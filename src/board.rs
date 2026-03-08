use embassy_stm32::rcc;
use embassy_stm32::Config;

/// 按 HARDWARE.md：HSE=8MHz，无 LSE，系统主频跑到该芯片规格允许的最大值。
///
/// 注意：embassy-stm32 的具体字段命名可能随版本略有差异；如后续升级依赖，
/// 若这里编译报错，需要按新版 API 调整（但约束不变）。
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
