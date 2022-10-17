#![no_std]

core::arch::global_asm! {
    ".global initialize_ns_data
     .thumb_func",
    "initialize_ns_data:",

    // Initialise .bss memory. `__sbss` and `__ebss` come from the linker script.
    "ldr r0, =__sbss
     ldr r1, =__ebss
     movs r2, #0
     0:
     cmp r1, r0
     beq 1f
     stm r0!, {{r2}}
     b 0b
     1:",

    // Initialise .data memory. `__sdata`, `__sidata`, and `__edata` come from the linker script.
    "ldr r0, =__sdata
     ldr r1, =__edata
     ldr r2, =__sidata
     2:
     cmp r1, r0
     beq 3f
     ldm r2!, {{r3}}
     stm r0!, {{r3}}
     b 2b
     3:",

    // Jump back to the caller.
    "bx lr",
}
