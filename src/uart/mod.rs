//! 串口驱动层（Embassy async + DMA）。
//!
//! ## 硬件约束（来自 HARDWARE.md）
//! - 4 路位置传感器：TTL `9600 8N1`，接收用 DMA + IDLE 分帧
//! - 上位机链路：USART1 → RS485，仅 TX（软件不启用 RX）
//!
//! ## 设计方式
//! - `init(...)` 采用“消耗外设所有权”的签名：把需要的 `peripherals::*` 和 DMA 通道
//!   作为参数传入，初始化后：
//!   - spawn 传感器 RX 任务（见 `sensor_ports::sensor_rx_task`）
//!   - 初始化上位机 TX-only 句柄（见 `host_tx::init`）
//! - 对上层暴露的接口尽量简单：
//!   - 传感器输入目前只保留 `on_sensor_frame(...)` 这个协议预留口
//!   - 上位机输出统一走 `host_tx::send_to_host(...)`
//!
//! 说明：本工程偏好“固定回调入口 + 静态函数”的风格，不使用显式 user_ctx 注入。

//! ## Beginner Notes（给 Rust/Embassy 初学者）
//! - **外设所有权（Ownership）**：`peripherals::USARTx`、DMA 通道、引脚都是“只能被初始化一次”的资源。
//!   把它们作为参数传给 `init(...)` 并在内部 `move` 进任务，可以让编译器保证：不会被重复使用。
//! - **为什么任务参数经常是 `'static`**：Embassy 的任务可能会运行到程序结束。
//!   因此任务持有的资源（例如 `UartRx<'static, _>`）需要满足足够长的生命周期。
//!   这也是为什么我们通常在初始化阶段把外设句柄构造成 `'static` 能用的形态。
//! - **IRQ 绑定**：`bind_interrupts!` 把具体的中断向量“绑定”到驱动需要的 handler 上。
//!   你可以把它理解成 HAL 内部需要的中断回调注册。

//! ## Reading Guide
//! - 先看 [`init`]：了解有哪些串口被初始化，以及后台任务在哪里 spawn。
//! - 再看 `sensor_ports::sensor_rx_task`：DMA+IDLE 分帧接收的核心循环。
//! - 最后看 `host_tx::{init, send_to_host}`：上位机 TX-only 的初始化与发送。

use embassy_executor::Spawner;
use embassy_stm32::{bind_interrupts, peripherals, usart};

pub mod host_tx;
pub mod sensor_ports;

bind_interrupts!(pub struct UartIrqs {
	UART4 => usart::InterruptHandler<peripherals::UART4>;
	USART2 => usart::InterruptHandler<peripherals::USART2>;
	USART3 => usart::InterruptHandler<peripherals::USART3>;
	USART6 => usart::InterruptHandler<peripherals::USART6>;
});

/// 初始化 UART/USART 外设并启动相关后台任务。
///
/// # 参数约束
/// - `uart4/usart2/usart3/usart6`：4 路位置传感器 RX
/// - `usart1`：上位机 TX-only
/// - `*_rx` / `*_rx_dma`：对应的 RX 引脚与 DMA 通道
/// - `usart1_tx` / `usart1_tx_dma`：上位机 TX 引脚与 DMA 通道
///
/// # 行为
/// - 配置串口参数为 `9600 8N1`
/// - 为 4 路传感器各 spawn 一个 `sensor_rx_task`
/// - 初始化 `host_tx` 的全局 TX 句柄（供 `send_to_host` 使用）
pub async fn init(
	spawner: Spawner,
	uart4: peripherals::UART4,
	uart4_rx: peripherals::PA1,
	uart4_rx_dma: peripherals::DMA1_CH2,
	usart2: peripherals::USART2,
	usart2_rx: peripherals::PA3,
	usart2_rx_dma: peripherals::DMA1_CH5,
	usart3: peripherals::USART3,
	usart3_rx: peripherals::PC11,
	usart3_rx_dma: peripherals::DMA1_CH1,
	usart6: peripherals::USART6,
	usart6_rx: peripherals::PC7,
	usart6_rx_dma: peripherals::DMA2_CH1,
	usart1: peripherals::USART1,
	usart1_tx: peripherals::PA9,
	usart1_tx_dma: peripherals::DMA2_CH7,
) {
	let mut cfg = usart::Config::default();
	cfg.baudrate = 9_600;
	cfg.data_bits = usart::DataBits::DataBits8;
	cfg.stop_bits = usart::StopBits::STOP1;
	cfg.parity = usart::Parity::ParityNone;

	// 4 路传感器 UART RX：DMA + IDLE 分帧
	let uart4_rx = usart::UartRx::new(uart4, UartIrqs, uart4_rx, uart4_rx_dma, cfg).unwrap();
	let usart2_rx = usart::UartRx::new(usart2, UartIrqs, usart2_rx, usart2_rx_dma, cfg).unwrap();
	let usart3_rx = usart::UartRx::new(usart3, UartIrqs, usart3_rx, usart3_rx_dma, cfg).unwrap();
	let usart6_rx = usart::UartRx::new(usart6, UartIrqs, usart6_rx, usart6_rx_dma, cfg).unwrap();

	spawner
		.spawn(sensor_ports::sensor_rx_task(
			uart4_rx,
			sensor_ports::SensorPortId::Uart4,
		))
		.unwrap();
	spawner
		.spawn(sensor_ports::sensor_rx_task(
			usart2_rx,
			sensor_ports::SensorPortId::Usart2,
		))
		.unwrap();
	spawner
		.spawn(sensor_ports::sensor_rx_task(
			usart3_rx,
			sensor_ports::SensorPortId::Usart3,
		))
		.unwrap();
	spawner
		.spawn(sensor_ports::sensor_rx_task(
			usart6_rx,
			sensor_ports::SensorPortId::Usart6,
		))
		.unwrap();

	// 上位机链路：USART1 → RS485（TX-only）
	host_tx::init(usart1, usart1_tx, usart1_tx_dma, cfg).await;
}
