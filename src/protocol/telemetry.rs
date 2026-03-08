//! 上位机上报帧（临时/最小实现）。
//!
//! 目标：提供一个“可用的、无堆分配”的状态上报编码器，
//! 让应用层按轮询节奏把输入快照发到 `uart::host_tx`。

/// 固定长度上报帧。
///
/// 格式（Little-endian）：
/// - magic: 0xA5 0x5A
/// - ver:   0x01
/// - seq:   u8
/// - axis_a: u16
/// - axis_b: u16
/// - axis_c: u16
/// - buttons_mask: u8 (bit0..bit4 对应 5 个按键，1=Pressed)
/// - checksum: u8 (对 ver..buttons_mask 做 XOR)
///
/// 说明：这是为了尽快联调的“最小帧”，后续你可以替换为正式协议。
pub const FRAME_LEN: usize = 12;

#[derive(Copy, Clone, Default, Eq, PartialEq, defmt::Format)]
pub struct TelemetryInputs {
	pub axis_a: u16,
	pub axis_b: u16,
	pub axis_c: u16,
	pub buttons_mask: u8,
}

/// 将输入编码为固定长度帧，返回写入的长度（恒为 [`FRAME_LEN`]）。
pub fn encode_inputs(seq: u8, inputs: TelemetryInputs, out: &mut [u8; FRAME_LEN]) -> usize {
	out[0] = 0xA5;
	out[1] = 0x5A;
	out[2] = 0x01; // ver
	out[3] = seq;

	let a = inputs.axis_a.to_le_bytes();
	let b = inputs.axis_b.to_le_bytes();
	let c = inputs.axis_c.to_le_bytes();
	out[4] = a[0];
	out[5] = a[1];
	out[6] = b[0];
	out[7] = b[1];
	out[8] = c[0];
	out[9] = c[1];
	out[10] = inputs.buttons_mask;

	let mut checksum = 0u8;
	for &x in &out[2..=10] {
		checksum ^= x;
	}
	out[11] = checksum;

	FRAME_LEN
}
