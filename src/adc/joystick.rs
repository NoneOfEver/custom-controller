//! 摇杆三轴 ADC 采样骨架。
//!
//! 约束来自 HARDWARE.md：
//! - ADC1: IN8(PB0) -> IN4(PA4) -> IN5(PA5)
//! - 输入 0–3.3V
//! - 软件触发 + 多通道扫描
//! - 采样时间/滤波/标定后续可调整

//! ## 数据模型
//! - 采样任务持续运行，维护一个“最新值快照”（`JOYSTICK_RAW`）。
//! - 上层以轮询方式调用 [`latest`] 获取最新值。
//!
//! ## 采样策略
//! - 使用 Embassy ADC v2 ring-buffer DMA：持续采样写入 DMA buffer。
//! - 每次 `read(...)` 返回一段“最近采样到的数据块”；这里取该块最后一组样本。
//! - 当前增加了 10ms 的节拍，避免无意义满速采样（后续可按联调需要调整）。

//! ## Beginner Notes
//! - `static` + DMA：DMA 需要长期有效的 buffer 地址；`StaticCell` 提供安全的一次性静态初始化。
//! - “快照”模式：任务写入 `Mutex<JoystickRaw>`，上层读取副本；这样能避免复杂的锁粒度和生命周期问题。
//! - `defmt::warn!`：嵌入式日志宏（类似 printf，但更适合 no_std/低开销）。

//! ## Reading Guide
//! - 先看 [`JoystickRaw`]：这是上层能读到的数据结构。
//! - 再看 [`latest`]：这是轮询接口（读快照）。
//! - 最后看 [`joystick_adc_task`]：它解释了 DMA buffer、采样顺序与更新快照的逻辑。

use embassy_stm32::adc::{self, SampleTime, Sequence};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

#[derive(Copy, Clone, Default, defmt::Format)]
/// 摇杆三轴的“原始 ADC 读数”。
///
/// 说明：这是未经标定/滤波的原始值（通常为 12-bit，存放在 `u16`）。
/// 字段与 ADC 通道顺序的映射在本模块固定（见 `joystick_adc_task` 的 sequence 设置）。
pub struct JoystickRaw {
    pub axis_a: u16,
    pub axis_b: u16,
    pub axis_c: u16,
}

static JOYSTICK_RAW: Mutex<CriticalSectionRawMutex, JoystickRaw> = Mutex::new(JoystickRaw {
    axis_a: 0,
    axis_b: 0,
    axis_c: 0,
});

#[allow(dead_code)]
/// 轮询读取“最新摇杆原始值快照”。
///
/// - 无阻塞 I/O；但会短暂进入互斥区读取结构体副本。
/// - 如果采样任务尚未运行，初始值为全 0。
pub async fn latest() -> JoystickRaw {
    *JOYSTICK_RAW.lock().await
}

#[embassy_executor::task]
/// ADC1 摇杆三轴采样后台任务。
///
/// # 行为概览
/// 1. 配置 ADC1 ring-buffer DMA
/// 2. 固定扫描顺序：IN8(PB0) → IN4(PA4) → IN5(PA5)
/// 3. 循环读取 DMA 半缓冲，并更新 `JOYSTICK_RAW`
pub async fn joystick_adc_task(
    adc1: embassy_stm32::peripherals::ADC1,
    dma: embassy_stm32::peripherals::DMA2_CH0,
    mut in8: embassy_stm32::peripherals::PB0,
    mut in4: embassy_stm32::peripherals::PA4,
    mut in5: embassy_stm32::peripherals::PA5,
) {
    const CHANNELS: usize = 3;
    const SEQ_SAMPLES: usize = 40;
    const DMA_BUF_LEN: usize = CHANNELS * SEQ_SAMPLES;

    static DMA_BUF: StaticCell<[u16; DMA_BUF_LEN]> = StaticCell::new();
    let dma_buf = DMA_BUF.init([0u16; DMA_BUF_LEN]);

    // Rust/嵌入式提示：
    // - 很多嵌入式工程不使用 heap（`no_std` 环境也可能没有全局分配器）。
    // - DMA 需要“稳定且长期有效”的内存地址；把 buffer 放在 `static` 区最稳妥。
    // - `StaticCell` 提供“一次性初始化”的静态内存槽：
    //   既能拿到 `&'static mut [u16; N]`，又能避免可变静态变量的 unsafe。

    let adc = adc::Adc::new(adc1);
    let mut adc = adc.into_ring_buffered(dma, dma_buf);

    // 按 HARDWARE.md 固定通道顺序：IN8 -> IN4 -> IN5
    adc.set_sample_sequence(Sequence::One, &mut in8, SampleTime::CYCLES112);
    adc.set_sample_sequence(Sequence::Two, &mut in4, SampleTime::CYCLES112);
    adc.set_sample_sequence(Sequence::Three, &mut in5, SampleTime::CYCLES112);

    let mut measurements = [0u16; DMA_BUF_LEN / 2];

    loop {
        match adc.read(&mut measurements).await {
            Ok(_n) => {
                // measurements 里包含多次序列采样；取最后一组 (IN8, IN4, IN5)
                let base = measurements.len() - CHANNELS;
                let raw = JoystickRaw {
                    axis_a: measurements[base],
                    axis_b: measurements[base + 1],
                    axis_c: measurements[base + 2],
                };

                *JOYSTICK_RAW.lock().await = raw;
            }
            Err(e) => {
                defmt::warn!("adc overrun: {:?}", e);
            }
        }

        // 先给一个保守采样节奏，避免无意义满速采样。
        embassy_time::Timer::after_millis(10).await;
    }
}
