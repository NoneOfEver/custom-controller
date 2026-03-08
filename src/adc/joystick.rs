//! 摇杆三轴 ADC 采样骨架。
//!
//! 约束来自 HARDWARE.md：
//! - ADC1: IN8(PB0) -> IN4(PA4) -> IN5(PA5)
//! - 输入 0–3.3V
//! - 软件触发 + 多通道扫描
//! - 采样时间/滤波/标定后续可调整

#[derive(Copy, Clone, Default, defmt::Format)]
pub struct JoystickRaw {
    pub axis_a: u16,
    pub axis_b: u16,
    pub axis_c: u16,
}
