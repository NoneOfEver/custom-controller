//! 数字输入（按键）模块入口。
//!
//! 当前实现：5 个按键的周期扫描 + 20ms 软件去抖。
//! 约束与引脚映射见工程根目录 `HARDWARE.md`。
//!
//! 设计方式：`init(...)` 消耗 GPIO 引脚外设所有权，并 spawn 后台任务。

//! ## Beginner Notes
//! - GPIO 引脚也遵循“所有权”：把 `PC0` 等引脚 move 进驱动后，就不会被别处误用。
//! - 为什么不做 EXTI：本工程选择最简单可靠的“周期扫描 + 去抖”，便于初学者理解与调试。

//! ## Reading Guide
//! - 只看一个入口：[`init`]（它会 spawn `buttons::buttons_task`）。
//! - 稳定态读取接口在 `buttons::latest()`。

use embassy_executor::Spawner;
use embassy_stm32::peripherals;

pub mod buttons;

/// 初始化按键扫描与去抖任务。
///
/// # 参数
/// - `button1..button4`：PC0..PC3
/// - `joystick_button`：PB7
///
/// # 行为
/// - spawn `buttons::buttons_task`
/// - 按键稳定态可通过 `buttons::latest()` 轮询读取
pub fn init(
	spawner: Spawner,
	button1: peripherals::PC0,
	button2: peripherals::PC1,
	button3: peripherals::PC2,
	button4: peripherals::PC3,
	joystick_button: peripherals::PB7,
) {
	spawner
		.spawn(buttons::buttons_task(
			button1,
			button2,
			button3,
			button4,
			joystick_button,
		))
		.unwrap();
}
