//! 应用层状态与整合占位。
//!
//! 建议后续在这里汇总：
//! - 4 路位置传感器状态
//! - 摇杆三轴值
//! - 按键状态
//! 并提供给上位机发送与本地逻辑使用。

//! ## 当前实现（联调优先）
//! - 通过 [`latest_inputs`] 轮询聚合输入快照（摇杆 + 按键稳定态）。
//! - 通过 [`host_report_task`] 以固定周期（20ms）把输入编码并从 USART1(TX-only) 发出。
//!
//! 说明：这里的实现刻意保持“简单可审阅”，便于后续替换为正式协议/更复杂的应用逻辑。

//! ## Beginner Notes
//! - 应用层的职责是“把各个驱动模块拼起来”：例如轮询输入、做策略、调用协议编码、触发发送。
//! - 这里选择“轮询快照”而不是事件推送：
//!   - 优点：更直观、易调试（像 C++ 里读全局状态）
//!   - 缺点：可能浪费一点 CPU/带宽（但对联调阶段通常足够）

//! ## Reading Guide
//! - 先看 [`InputsSnapshot`]：应用层聚合后的输入结构。
//! - 再看 [`latest_inputs`]：轮询聚合接口。
//! - 最后看 [`host_report_task`]：它展示了“轮询 → 编码 → 发送”的完整闭环。

use crate::adc::joystick::JoystickRaw;
use crate::input::buttons::ButtonState;
use crate::input::buttons::ButtonsSnapshot;
use crate::protocol::telemetry::TelemetryInputs;

#[derive(Copy, Clone, Default, defmt::Format)]
/// 应用层视角的“输入快照”。
///
/// - `joystick`：三轴 ADC 原始值（来自 ADC 采样任务）
/// - `buttons`：5 个按键稳定态（来自按键去抖任务）
pub struct InputsSnapshot {
	pub joystick: JoystickRaw,
	pub buttons: ButtonsSnapshot,
}

/// 轮询式读取输入快照（按键稳定态 + 摇杆最新值）。
///
/// 说明：本工程倾向“上层按需轮询”，不做按键事件推送。
#[allow(dead_code)]
pub async fn latest_inputs() -> InputsSnapshot {
	InputsSnapshot {
		joystick: crate::adc::joystick::latest().await,
		buttons: crate::input::buttons::latest().await,
	}
}

fn is_pressed(state: ButtonState) -> bool {
	// `matches!` 是一个宏：类似 C++ 里对 enum 做模式匹配。
	matches!(state, ButtonState::Pressed)
}

fn buttons_mask(buttons: ButtonsSnapshot) -> u8 {
	// 位图（bitmask）是嵌入式里常用的“紧凑表示法”。
	// 这里约定：bit=1 表示 Pressed。
	let mut mask = 0u8;
	if is_pressed(buttons.button1) {
		mask |= 1 << 0;
	}
	if is_pressed(buttons.button2) {
		mask |= 1 << 1;
	}
	if is_pressed(buttons.button3) {
		mask |= 1 << 2;
	}
	if is_pressed(buttons.button4) {
		mask |= 1 << 3;
	}
	if is_pressed(buttons.joystick) {
		mask |= 1 << 4;
	}
	mask
}

/// 最小联调闭环：轮询输入快照 → 编码 → 通过 USART1(TX-only) 发给上位机。
///
/// 后续你可以把它替换为正式协议/更合适的发送策略。
///
/// # 前置条件
/// - `uart::init(...).await` 必须先完成（它会初始化 `uart::host_tx`）
/// - `adc::init(...)` 与 `input::init(...)` 已 spawn 对应后台任务
#[embassy_executor::task]
pub async fn host_report_task() {
	const PERIOD_MS: u64 = 20;

	let mut seq = 0u8;
	let mut frame = [0u8; crate::protocol::telemetry::FRAME_LEN];

	loop {
		let inputs = latest_inputs().await;
		// Rust 的 struct 初始化支持“字段名: 值”，和 C 的 designated initializer 类似。
		let telemetry = TelemetryInputs {
			axis_a: inputs.joystick.axis_a,
			axis_b: inputs.joystick.axis_b,
			axis_c: inputs.joystick.axis_c,
			buttons_mask: buttons_mask(inputs.buttons),
		};

		let n = crate::protocol::telemetry::encode_inputs(seq, telemetry, &mut frame);
		// `&frame[..n]` 是切片（slice）：指向同一段数组内存的“视图”，不拷贝。
		crate::uart::host_tx::send_to_host(&frame[..n]).await;

		// `wrapping_add` 表示 0..255 溢出时自动回到 0（环形计数器）。
		seq = seq.wrapping_add(1);
		embassy_time::Timer::after_millis(PERIOD_MS).await;
	}
}
