//! 4 路位置传感器 UART 接收骨架（DMA + IDLE）。
//!
//! 约束来自 HARDWARE.md：
//! - UART4  (PA0/PA1)
//! - USART2 (PA2/PA3)
//! - USART3 (PC10/PC11)
//! - USART6 (PC6/PC7)
//! - TTL, 9600 8N1
//! - 接收：DMA + IDLE 分帧
//!
//! 协议解析由你后续补充：这里仅提供“帧 bytes 到上层”的接口预留。

//! ## 关键点
//! - 使用 `UartRx::read_until_idle`：DMA 接收，遇到“总线空闲”即认为一帧结束。
//! - 这里的“帧”只是物理层分帧：协议层仍需校验/拼包/解析。
//! - `buf` 的长度就是单次分帧最大长度；超出会被截断（需在协议层处理）。

//! ## Beginner Notes
//! - `&[u8]`（slice）：函数参数用切片表示“一段连续 bytes 的只读视图”，不拷贝数据。
//! - `&buf[..n]`：对数组做切片，得到前 `n` 个字节。
//! - `match`：Rust 的模式匹配，相当于更强的 `switch`。

//! ## Reading Guide
//! - 先看 [`SensorPortId`]：它告诉你是哪一路物理串口。
//! - 再看 [`sensor_rx_task`]：它是分帧接收的主循环。
//! - 最后看 [`on_sensor_frame`]：协议层未来会从这里接走 bytes。

use embassy_stm32::mode::Async;
use embassy_stm32::usart::UartRx;

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
pub enum SensorPortId {
    Uart4,
    Usart2,
    Usart3,
    Usart6,
}

/// 协议层接口占位：收到一帧 bytes 时调用。
///
/// 后续你可以把它改为 trait + 实现，或改成 channel/queue 推送。
///
/// # 参数
/// - `port`：标识来自哪一路串口
/// - `frame`：本次 `read_until_idle` 得到的原始 bytes（长度不超过内部 buffer）
pub fn on_sensor_frame(_port: SensorPortId, _frame: &[u8]) {
    // 占位：协议由你后补
}

#[embassy_executor::task]
/// 传感器串口 RX 后台任务。
///
/// 任务会无限循环：读取一段 bytes（直到 IDLE）并回调 [`on_sensor_frame`]。
pub async fn sensor_rx_task(mut rx: UartRx<'static, Async>, port: SensorPortId) {
    // 这里的 buffer 是“分帧最大长度”的上限。
    // 如果后续协议帧更长，可以再调大。
    // Rust 里 `[0u8; 128]` 表示“长度为 128 的数组”，元素类型是 u8，初始值全 0。
    let mut buf = [0u8; 128];

    loop {
        match rx.read_until_idle(&mut buf).await {
            Ok(0) => {}
            Ok(n) => on_sensor_frame(port, &buf[..n]),
            Err(e) => {
                defmt::warn!("{:?} rx error: {:?}", port, e);
            }
        }
    }
}
