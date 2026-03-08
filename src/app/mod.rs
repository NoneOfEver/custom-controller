//! 应用层状态与整合占位。
//!
//! 建议后续在这里汇总：
//! - 4 路位置传感器状态
//! - 摇杆三轴值
//! - 按键状态
//! 并提供给上位机发送与本地逻辑使用。

use crate::adc::joystick::JoystickRaw;
use crate::input::buttons::ButtonState;
use crate::input::buttons::ButtonsSnapshot;
use crate::protocol::telemetry::TelemetryInputs;

#[derive(Copy, Clone, Default, defmt::Format)]
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
	matches!(state, ButtonState::Pressed)
}

fn buttons_mask(buttons: ButtonsSnapshot) -> u8 {
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
#[embassy_executor::task]
pub async fn host_report_task() {
	const PERIOD_MS: u64 = 20;

	let mut seq = 0u8;
	let mut frame = [0u8; crate::protocol::telemetry::FRAME_LEN];

	loop {
		let inputs = latest_inputs().await;
		let telemetry = TelemetryInputs {
			axis_a: inputs.joystick.axis_a,
			axis_b: inputs.joystick.axis_b,
			axis_c: inputs.joystick.axis_c,
			buttons_mask: buttons_mask(inputs.buttons),
		};

		let n = crate::protocol::telemetry::encode_inputs(seq, telemetry, &mut frame);
		crate::uart::host_tx::send_to_host(&frame[..n]).await;

		seq = seq.wrapping_add(1);
		embassy_time::Timer::after_millis(PERIOD_MS).await;
	}
}
