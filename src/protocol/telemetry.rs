//! 上位机上报帧（临时/最小实现）。
//!
//! 目标：提供一个“可用的、无堆分配”的状态上报编码器，
//! 让应用层按轮询节奏把输入快照发到 `uart::host_tx`。

//! ## 适用范围
//! - 这是“联调优先”的最小协议：固定长度、字段明确、实现简单。
//! - 未来如果需要更强的鲁棒性（包头转义、长度字段、CRC、版本协商等），
//!   建议在保持 `protocol`/`app` 分层的前提下替换此模块。

//! ## Reading Guide
//! - 先看常量 [`FRAME_LEN`]：知道帧的固定长度。
//! - 再看 [`TelemetryInputs`]：它是“准备上报的数据结构”。
//! - 最后看 [`encode_inputs`]：它把字段写成字节流，并计算 checksum。

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
	/// 摇杆 Axis-A 原始 ADC 读数。
	pub axis_a: u16,
	/// 摇杆 Axis-B 原始 ADC 读数。
	pub axis_b: u16,
	/// 摇杆 Axis-C 原始 ADC 读数。
	pub axis_c: u16,
	/// 5 个按键位图（bit0..bit4），1=Pressed。
	pub buttons_mask: u8,
}

/// 将输入编码为固定长度帧，返回写入的长度（恒为 [`FRAME_LEN`]）。
///
/// # 参数
/// - `seq`：包序号（0..255 循环）
/// - `inputs`：待上报的输入字段（轴值 + 按键位图）
/// - `out`：输出缓冲（固定 12 bytes）
pub fn encode_inputs(seq: u8, inputs: TelemetryInputs, out: &mut [u8; FRAME_LEN]) -> usize {
	out[0] = 0xA5;
	out[1] = 0x5A;
	out[2] = 0x01; // ver
	out[3] = seq;

	// `to_le_bytes()` 会把整数转换成 little-endian 字节数组。
	// 这相当于你在 C 里手写：low byte / high byte。
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

	// 这里用 XOR 做一个非常轻量的校验（不是强校验，只用于联调快速排错）。
	let mut checksum = 0u8;
	for &x in &out[2..=10] {
		// `for &x in slice`：遍历切片里的每个 u8。
		checksum ^= x;
	}
	out[11] = checksum;

	FRAME_LEN
}
