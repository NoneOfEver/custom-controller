//! 按键扫描与去抖骨架。
//!
//! 约束来自 HARDWARE.md：
//! - BUTTON1..4: PC0..PC3
//! - JOYSTICK_BUTTON: PB7
//! - 板级上拉，低有效
//! - MCU 侧 no-pull
//! - 周期扫描（不使用 EXTI）
//! - 去抖：20ms

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
pub enum ButtonId {
    Button1,
    Button2,
    Button3,
    Button4,
    Joystick,
}

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
pub enum ButtonState {
    Released,
    Pressed,
}
