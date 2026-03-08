//! 摇杆三轴 ADC 采样骨架。
//!
//! 约束来自 HARDWARE.md：
//! - ADC1: IN8(PB0) -> IN4(PA4) -> IN5(PA5)
//! - 输入 0–3.3V
//! - 软件触发 + 多通道扫描
//! - 采样时间/滤波/标定后续可调整

use embassy_stm32::adc::{self, SampleTime, Sequence};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

#[derive(Copy, Clone, Default, defmt::Format)]
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
pub async fn latest() -> JoystickRaw {
    *JOYSTICK_RAW.lock().await
}

#[embassy_executor::task]
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
