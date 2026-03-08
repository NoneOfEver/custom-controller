//! 板级配置（当前只包含时钟/RCC）。
//!
//! 这里把 `HARDWARE.md` 中的时钟约束落地到 `embassy_stm32::Config`：
//! - HSE = 8MHz
//! - 不使用 LSE
//! - SYSCLK 目标 180MHz（以 STM32F446 规格为准）
//!
//! 备注：这类配置在 Embassy/HAL 升级时最容易发生 API 字段名变化。
//! 如果未来升级 `embassy-stm32` 后这里不编译，通常只需要按新版字段重写，
//! 但“时钟约束本身”不应改变。

//! ## Beginner Notes
//! - 这一文件主要是“把 datasheet/硬件约束变成代码”。
//! - 时钟配置通常是嵌入式最容易踩坑的部分：
//!   - PLL 分频倍频要满足芯片限制
//!   - APB 分频影响定时器/外设时钟
//! - 你可以先不纠结每个字段含义：知道它们来自“固定的硬件事实”即可。

//! ## Reading Guide
//! - 只需要重点看 [`apply_clock_config`]：它把 HSE/PLL/APB 的约束落地。
//! - 如果你以后要改系统频率：通常只需要改 PLL 与 APB 分频，并保持满足芯片规格。

use embassy_stm32::rcc;
use embassy_stm32::Config;

/// 应用板级时钟配置。
///
/// - 输入：`config` 是 `embassy_stm32::init` 使用的配置对象。
/// - 输出：函数会修改 `config.rcc.*` 字段。
///
/// 设计点：把“硬件事实（HSE=8MHz、无 LSE、目标 180MHz）”集中在此处，
/// 避免分散在 `main` 或各驱动模块里。
pub fn apply_clock_config(config: &mut Config) {
    use embassy_stm32::time::Hertz;

    // 外部高速晶振 8MHz
    config.rcc.hse = Some(rcc::Hse {
        freq: Hertz(8_000_000),
        mode: rcc::HseMode::Oscillator,
    });

    // 不使用 LSE/LSI（不启用 RTC 低速时钟）
    config.rcc.ls = rcc::LsConfig::off();

    // 目标：系统主频 180MHz（STM32F446 常见最大值；以芯片规格为准）
    // 选择：HSE=8MHz, PLLM=4, PLLN=180, PLLP=2
    //   VCO_in  = 8/4   = 2MHz
    //   VCO_out = 2*180 = 360MHz
    //   SYSCLK  = 360/2 = 180MHz
    config.rcc.pll_src = rcc::PllSource::HSE;
    config.rcc.pll = Some(rcc::Pll {
        prediv: rcc::PllPreDiv::DIV4,
        mul: rcc::PllMul::MUL180,
        divp: Some(rcc::PllPDiv::DIV2),
        divq: None,
        divr: None,
    });
    config.rcc.sys = rcc::Sysclk::PLL1_P;

    // 总线分频（典型配置）：
    //   AHB  = 180MHz
    //   APB1 = 45MHz（DIV4）
    //   APB2 = 90MHz（DIV2）
    config.rcc.ahb_pre = rcc::AHBPrescaler::DIV1;
    config.rcc.apb1_pre = rcc::APBPrescaler::DIV4;
    config.rcc.apb2_pre = rcc::APBPrescaler::DIV2;
}
