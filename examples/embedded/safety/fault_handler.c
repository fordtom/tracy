/**
 * @file fault_handler.c
 * @brief CPU Fault Handler for Safety-Critical Systems
 *
 * Implements fault detection and safe state transition for
 * HardFault, MemManage, BusFault, and UsageFault exceptions.
 *
 * @par DO-178C Compliance
 * - DAL A: Catastrophic failure prevention
 *
 * @par Safety Requirements
 * - SYS-SAF-200: All CPU faults shall be captured and logged
 * - SYS-SAF-201: System shall enter safe state on unrecoverable fault
 */

#include "fault_handler.h"
#include "safe_state.h"
#include "nvm_log.h"
#include "crc32.h"
#include <stdint.h>
#include <stdbool.h>

/**
 * @brief Fault status register snapshot
 *
 * SAF-FAULT-001: Capture all relevant registers on fault
 */
typedef struct {
    uint32_t r0;
    uint32_t r1;
    uint32_t r2;
    uint32_t r3;
    uint32_t r12;
    uint32_t lr;
    uint32_t pc;         /**< SAF-FAULT-002: Program counter at fault */
    uint32_t psr;
    uint32_t cfsr;       /**< SAF-FAULT-003: Configurable Fault Status */
    uint32_t hfsr;       /**< SAF-FAULT-004: HardFault Status */
    uint32_t dfsr;       /**< SAF-FAULT-005: Debug Fault Status */
    uint32_t mmfar;      /**< SAF-FAULT-006: MemManage Fault Address */
    uint32_t bfar;       /**< SAF-FAULT-007: BusFault Address */
    uint32_t afsr;       /**< Auxiliary Fault Status */
    uint32_t timestamp;  /**< SAF-FAULT-008: Fault timestamp */
    uint32_t crc;        /**< SAF-FAULT-009: Data integrity check */
} fault_record_t;

/**
 * @brief Fault classification
 *
 * SAF-FAULT-010: Faults shall be classified by severity
 */
typedef enum {
    FAULT_CLASS_RECOVERABLE = 0,  /**< Can attempt recovery */
    FAULT_CLASS_DEGRADED,         /**< Continue with reduced capability */
    FAULT_CLASS_FATAL             /**< Must enter safe state */
} fault_class_t;

/** SAF-FAULT-011: NVM storage for fault records */
#define FAULT_LOG_MAX_ENTRIES  16
#define FAULT_LOG_NVM_ADDR     0x0803F000

static volatile fault_record_t g_fault_log[FAULT_LOG_MAX_ENTRIES] __attribute__((section(".noinit")));
static volatile uint32_t g_fault_count __attribute__((section(".noinit")));

/**
 * @brief Analyze CFSR to determine fault type
 *
 * LLR-FAULT-001: Decode Configurable Fault Status Register
 *
 * @param cfsr CFSR register value
 * @return Fault classification
 */
static fault_class_t analyze_cfsr(uint32_t cfsr)
{
    // LLR-FAULT-002: Check for memory management faults
    if (cfsr & 0xFF) {  // MMFSR bits
        if (cfsr & (1 << 0)) {  // IACCVIOL
            return FAULT_CLASS_FATAL;  // SAF-FAULT-020: Instruction access violation is fatal
        }
        if (cfsr & (1 << 1)) {  // DACCVIOL
            return FAULT_CLASS_FATAL;  // SAF-FAULT-021: Data access violation is fatal
        }
    }

    // LLR-FAULT-003: Check for bus faults
    if (cfsr & 0xFF00) {  // BFSR bits
        if (cfsr & (1 << 8)) {  // IBUSERR
            return FAULT_CLASS_FATAL;  // SAF-FAULT-022: Instruction bus error is fatal
        }
        if (cfsr & (1 << 9)) {  // PRECISERR
            return FAULT_CLASS_DEGRADED;  // SAF-FAULT-023: Precise bus error may be recoverable
        }
        if (cfsr & (1 << 10)) {  // IMPRECISERR
            return FAULT_CLASS_DEGRADED;  // SAF-FAULT-024: Imprecise bus error
        }
    }

    // LLR-FAULT-004: Check for usage faults
    if (cfsr & 0xFFFF0000) {  // UFSR bits
        if (cfsr & (1 << 16)) {  // UNDEFINSTR
            return FAULT_CLASS_FATAL;  // SAF-FAULT-025: Undefined instruction
        }
        if (cfsr & (1 << 17)) {  // INVSTATE
            return FAULT_CLASS_FATAL;  // SAF-FAULT-026: Invalid state
        }
        if (cfsr & (1 << 18)) {  // INVPC
            return FAULT_CLASS_FATAL;  // SAF-FAULT-027: Invalid PC load
        }
        if (cfsr & (1 << 24)) {  // DIVBYZERO
            return FAULT_CLASS_RECOVERABLE;  // SAF-FAULT-028: Div by zero may be recoverable
        }
    }

    return FAULT_CLASS_FATAL;  // Default to fatal for unknown faults
}

/**
 * @brief Store fault record to NVM
 *
 * SAF-FAULT-030: Fault records shall persist across resets
 * SAF-FAULT-031: Records shall be integrity protected
 *
 * @param record Pointer to fault record
 */
static void store_fault_record(const fault_record_t *record)
{
    uint32_t index = g_fault_count % FAULT_LOG_MAX_ENTRIES;

    // LLR-FAULT-010: Copy record to NVM-backed RAM
    g_fault_log[index] = *record;

    // LLR-FAULT-011: Calculate and store CRC
    g_fault_log[index].crc = crc32_calculate(
        (const uint8_t *)&g_fault_log[index],
        sizeof(fault_record_t) - sizeof(uint32_t)
    );

    g_fault_count++;

    // LLR-FAULT-012: Trigger NVM flush
    nvm_flush_async();
}

/**
 * @brief Common fault handler implementation
 *
 * SAF-FAULT-040: Single entry point for all fault types
 *
 * @param stack_frame Pointer to exception stack frame
 * @param fault_type Fault type identifier
 */
void fault_handler_common(uint32_t *stack_frame, uint32_t fault_type)
{
    fault_record_t record;

    // SAF-FAULT-041: Capture CPU registers from stack frame
    record.r0  = stack_frame[0];
    record.r1  = stack_frame[1];
    record.r2  = stack_frame[2];
    record.r3  = stack_frame[3];
    record.r12 = stack_frame[4];
    record.lr  = stack_frame[5];
    record.pc  = stack_frame[6];
    record.psr = stack_frame[7];

    // SAF-FAULT-042: Capture fault status registers
    record.cfsr  = SCB->CFSR;
    record.hfsr  = SCB->HFSR;
    record.dfsr  = SCB->DFSR;
    record.mmfar = SCB->MMFAR;
    record.bfar  = SCB->BFAR;
    record.afsr  = SCB->AFSR;
    record.timestamp = get_system_tick();

    // SAF-FAULT-043: Store fault record
    store_fault_record(&record);

    // SAF-FAULT-044: Classify fault severity
    fault_class_t fault_class = analyze_cfsr(record.cfsr);

    // SAF-FAULT-045: Clear fault status bits
    SCB->CFSR = record.cfsr;
    SCB->HFSR = record.hfsr;

    // SAF-FAULT-046: Take appropriate action based on classification
    switch (fault_class) {
        case FAULT_CLASS_RECOVERABLE:
            // LLR-FAULT-020: Attempt recovery by skipping faulting instruction
            stack_frame[6] += 2;  // Skip to next instruction (Thumb)
            return;

        case FAULT_CLASS_DEGRADED:
            // LLR-FAULT-021: Enter degraded mode
            enter_degraded_mode(DEGRADE_REASON_BUS_FAULT);
            return;

        case FAULT_CLASS_FATAL:
        default:
            // LLR-FAULT-022: Enter safe state
            enter_safe_state(SAFE_STATE_CPU_FAULT);
            break;
    }

    // SAF-FAULT-047: Should not reach here - force reset
    NVIC_SystemReset();
}

/**
 * @brief HardFault exception handler
 *
 * SYS-SAF-200: Capture HardFault exceptions
 *
 * @note Naked function - no prologue/epilogue
 */
__attribute__((naked))
void HardFault_Handler(void)
{
    __asm volatile(
        "tst lr, #4         \n"  // LLR-FAULT-030: Determine stack in use
        "ite eq             \n"
        "mrseq r0, msp      \n"  // Using MSP
        "mrsne r0, psp      \n"  // Using PSP
        "mov r1, #0         \n"  // Fault type = HardFault
        "b fault_handler_common \n"
    );
}

/**
 * @brief MemManage fault handler
 *
 * SAF-FAULT-050: Memory protection violations
 */
__attribute__((naked))
void MemManage_Handler(void)
{
    __asm volatile(
        "tst lr, #4         \n"
        "ite eq             \n"
        "mrseq r0, msp      \n"
        "mrsne r0, psp      \n"
        "mov r1, #1         \n"  // Fault type = MemManage
        "b fault_handler_common \n"
    );
}

/**
 * @brief BusFault handler
 *
 * SAF-FAULT-051: Bus access errors
 */
__attribute__((naked))
void BusFault_Handler(void)
{
    __asm volatile(
        "tst lr, #4         \n"
        "ite eq             \n"
        "mrseq r0, msp      \n"
        "mrsne r0, psp      \n"
        "mov r1, #2         \n"  // Fault type = BusFault
        "b fault_handler_common \n"
    );
}

/**
 * @brief UsageFault handler
 *
 * SAF-FAULT-052: Invalid instruction/state faults
 */
__attribute__((naked))
void UsageFault_Handler(void)
{
    __asm volatile(
        "tst lr, #4         \n"
        "ite eq             \n"
        "mrseq r0, msp      \n"
        "mrsne r0, psp      \n"
        "mov r1, #3         \n"  // Fault type = UsageFault
        "b fault_handler_common \n"
    );
}

/**
 * @brief Get fault log for diagnostics
 *
 * SAF-FAULT-060: Fault history shall be readable
 *
 * @param buffer Output buffer for fault records
 * @param max_count Maximum records to retrieve
 * @return Number of records copied
 */
uint32_t fault_get_log(fault_record_t *buffer, uint32_t max_count)
{
    uint32_t count = (g_fault_count < max_count) ? g_fault_count : max_count;

    for (uint32_t i = 0; i < count; i++) {
        uint32_t index = (g_fault_count - count + i) % FAULT_LOG_MAX_ENTRIES;

        // SAF-FAULT-061: Verify record integrity
        uint32_t expected_crc = crc32_calculate(
            (const uint8_t *)&g_fault_log[index],
            sizeof(fault_record_t) - sizeof(uint32_t)
        );

        if (g_fault_log[index].crc == expected_crc) {
            buffer[i] = g_fault_log[index];
        } else {
            // SAF-FAULT-062: Mark corrupted records
            buffer[i].pc = 0xDEADBEEF;
            buffer[i].timestamp = 0;
        }
    }

    return count;
}

/**
 * @brief Clear fault log
 *
 * SAF-FAULT-063: Log clear requires explicit action
 */
void fault_clear_log(void)
{
    g_fault_count = 0;
    for (uint32_t i = 0; i < FAULT_LOG_MAX_ENTRIES; i++) {
        g_fault_log[i].crc = 0;  // Invalidate all records
    }
}
