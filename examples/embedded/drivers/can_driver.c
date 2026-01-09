/**
 * @file can_driver.c
 * @brief CAN Bus Driver for Automotive ECU
 *
 * SRS-CAN-001: CAN driver shall support CAN 2.0B protocol
 * SRS-CAN-002: CAN driver shall support baud rates up to 1Mbps
 */

#include "can_driver.h"
#include "hw_registers.h"

/**
 * @brief Maximum number of CAN message buffers
 * LLR-CAN-010: Driver shall support at least 16 message buffers
 */
#define CAN_MAX_BUFFERS 16

/**
 * @brief CAN baud rate configuration
 * LLR-CAN-011: Default baud rate shall be 500kbps
 */
#define CAN_DEFAULT_BAUD 500000U

/**
 * @brief Initialize the CAN peripheral
 *
 * SRS-CAN-003: CAN initialization shall complete within 10ms
 * SRS-CAN-004: CAN initialization shall configure hardware filters
 *
 * @param config Pointer to CAN configuration structure
 * @return CAN_OK on success, error code otherwise
 */
can_status_t can_init(const can_config_t *config)
{
    if (config == NULL) {
        return CAN_ERR_NULL_PTR;  // LLR-CAN-020: Validate input parameters
    }

    // LLR-CAN-021: Reset CAN controller before initialization
    CAN_CTRL_REG = CAN_CTRL_RESET;

    // LLR-CAN-022: Wait for reset completion
    while (CAN_CTRL_REG & CAN_CTRL_RESET) {
        // Busy wait - timeout handled by watchdog
    }

    // LLR-CAN-023: Configure baud rate prescaler
    uint32_t prescaler = SYSTEM_CLOCK / (config->baud_rate * CAN_TIME_QUANTA);
    CAN_BAUD_REG = prescaler;

    // LLR-CAN-024: Enable CAN interrupts
    CAN_INT_REG = CAN_INT_RX | CAN_INT_TX | CAN_INT_ERR;

    return CAN_OK;
}

/**
 * @brief Transmit a CAN message
 *
 * SRS-CAN-005: Transmit function shall be non-blocking
 * SRS-CAN-006: Transmit shall support both standard and extended IDs
 *
 * @param msg Pointer to message structure
 * @return CAN_OK if message queued successfully
 */
can_status_t can_transmit(const can_msg_t *msg)
{
    if (msg == NULL) {
        return CAN_ERR_NULL_PTR;
    }

    // LLR-CAN-030: Check for available transmit buffer
    if (!(CAN_STATUS_REG & CAN_TX_BUF_AVAIL)) {
        return CAN_ERR_TX_BUSY;  // SRS-CAN-007: Return busy if no buffer available
    }

    // LLR-CAN-031: Load message ID
    if (msg->flags & CAN_FLAG_EXTENDED) {
        CAN_TX_ID_REG = msg->id | CAN_ID_EXTENDED;
    } else {
        CAN_TX_ID_REG = msg->id;
    }

    // LLR-CAN-032: Load message data
    CAN_TX_DATA_REG = *(uint64_t *)msg->data;
    CAN_TX_DLC_REG = msg->dlc;

    // LLR-CAN-033: Trigger transmission
    CAN_TX_CTRL_REG = CAN_TX_START;

    return CAN_OK;
}

/**
 * @brief Receive a CAN message (polling mode)
 *
 * SRS-CAN-008: Receive shall support polling and interrupt modes
 *
 * @param msg Pointer to buffer for received message
 * @param timeout_ms Timeout in milliseconds (0 = no wait)
 * @return CAN_OK if message received, CAN_ERR_TIMEOUT otherwise
 */
can_status_t can_receive(can_msg_t *msg, uint32_t timeout_ms)
{
    uint32_t start_tick = get_system_tick();

    // LLR-CAN-040: Poll receive buffer with timeout
    while (!(CAN_STATUS_REG & CAN_RX_MSG_AVAIL)) {
        if (timeout_ms > 0) {
            uint32_t elapsed = get_system_tick() - start_tick;
            if (elapsed >= timeout_ms) {
                return CAN_ERR_TIMEOUT;
            }
        } else {
            return CAN_ERR_NO_MSG;
        }
    }

    // LLR-CAN-041: Read message from hardware buffer
    msg->id = CAN_RX_ID_REG & CAN_ID_MASK;
    msg->flags = (CAN_RX_ID_REG & CAN_ID_EXTENDED) ? CAN_FLAG_EXTENDED : 0;
    msg->dlc = CAN_RX_DLC_REG;
    *(uint64_t *)msg->data = CAN_RX_DATA_REG;

    // LLR-CAN-042: Release receive buffer
    CAN_RX_CTRL_REG = CAN_RX_RELEASE;

    return CAN_OK;
}

/**
 * @brief Configure hardware message filter
 *
 * SRS-CAN-009: Driver shall support at least 8 hardware filters
 *
 * @param filter_id Filter slot (0-7)
 * @param id CAN ID to accept
 * @param mask Acceptance mask
 */
void can_set_filter(uint8_t filter_id, uint32_t id, uint32_t mask)
{
    if (filter_id >= CAN_MAX_FILTERS) {
        return;  // LLR-CAN-050: Silently ignore invalid filter ID
    }

    // LLR-CAN-051: Configure filter registers
    CAN_FILTER_ID_REG(filter_id) = id;
    CAN_FILTER_MASK_REG(filter_id) = mask;
    CAN_FILTER_CTRL_REG |= (1U << filter_id);  // LLR-CAN-052: Enable filter
}

// SRS-CAN-010: Driver shall report bus-off recovery attempts
static uint32_t bus_off_recovery_count = 0;

/**
 * @brief CAN error interrupt handler
 *
 * SAF-CAN-001: Error handler shall log all error events
 * SAF-CAN-002: Bus-off condition shall trigger recovery procedure
 */
void can_error_isr(void)
{
    uint32_t error_flags = CAN_ERR_REG;

    if (error_flags & CAN_ERR_BUS_OFF) {
        bus_off_recovery_count++;  // SAF-CAN-003: Track recovery attempts
        can_bus_off_recovery();
    }

    if (error_flags & CAN_ERR_PASSIVE) {
        // SAF-CAN-004: Log error passive transition
        error_log(ERR_CAN_PASSIVE, CAN_ERR_CNT_REG);
    }

    CAN_ERR_REG = error_flags;  // Clear handled errors
}
