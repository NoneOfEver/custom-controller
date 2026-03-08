/* STM32F446RETx (RET6) 内存布局（常见配置）
 * - FLASH: 512K @ 0x0800_0000
 * - RAM:   128K @ 0x2000_0000
 *
 * 注：部分 F4 还有 CCM RAM（0x1000_0000），这里先不使用；
 * 如后续需要把 DMA buffer 放主 RAM、CPU 计算放 CCM，可再拆分。
 */

MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 512K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
