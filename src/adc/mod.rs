//! ADC 驱动层入口。
//!
//! 当前只包含摇杆三轴采样（ADC1 三通道），细节见 `joystick` 子模块。
//!
//! 设计方式：`init(...)` 消耗 ADC/DMA/PIN 外设所有权，并 spawn 后台任务。

//! ## Beginner Notes
//! - ADC 采样是“持续后台工作”：把它做成任务后，上层只需要轮询最新值。
//! - `Spawner`：Embassy 用来启动任务的“任务启动器”。
//!   可以把它类比成 RTOS 里创建线程/任务的 API。

//! ## Reading Guide
//! - 只看一个入口：[`init`]。
//! - 采样细节与数据结构在 `joystick` 子模块（`joystick_adc_task` / `latest`）。

use embassy_executor::Spawner;
use embassy_stm32::peripherals;

pub mod joystick;

/// 初始化 ADC1 摇杆采样任务。
///
/// # 参数
/// - `adc1`：ADC1 外设
/// - `dma`：DMA2_CH0（按当前板级分配）
/// - `in8/in4/in5`：对应 PB0/PA4/PA5（通道顺序在 `joystick` 内固定）
///
/// # 行为
/// - spawn 一个后台任务持续采样
/// - 采样结果存入全局“最新值快照”（见 `joystick::latest`）
pub fn init(
	spawner: Spawner,
	adc1: peripherals::ADC1,
	dma: peripherals::DMA2_CH0,
	in8: peripherals::PB0,
	in4: peripherals::PA4,
	in5: peripherals::PA5,
) {
	spawner.spawn(joystick::joystick_adc_task(adc1, dma, in8, in4, in5)).unwrap();
}
