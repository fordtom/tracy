/**
 * @file lin_driver.h
 * @brief LIN Bus Driver for Automotive Body Control
 *
 * Implements LIN 2.1 protocol for body electronics communication.
 *
 * @par Applicable Standards
 * - LIN Specification 2.1
 * - ISO 17987 (LIN)
 *
 * @par Requirements Document
 * SWRD-LIN: LIN Bus Driver Software Requirements
 */

#ifndef LIN_DRIVER_H
#define LIN_DRIVER_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/*============================================================================
 * Constants and Macros
 *===========================================================================*/

/**
 * SWRD-LIN-001: LIN baud rate shall be configurable
 * @{
 */
#define LIN_BAUD_9600   9600U
#define LIN_BAUD_19200  19200U   /**< Standard automotive baud rate */
/** @} */

/**
 * SWRD-LIN-002: Maximum message data length
 */
#define LIN_MAX_DATA_LEN  8U

/**
 * SWRD-LIN-003: Number of message slots for schedule table
 */
#define LIN_MAX_SCHEDULE_SLOTS  64U

/**
 * LLR-LIN-001: Break field timing
 */
#define LIN_BREAK_BITS  13U

/**
 * LLR-LIN-002: Sync field value
 */
#define LIN_SYNC_BYTE  0x55U

/*============================================================================
 * Type Definitions
 *===========================================================================*/

/**
 * @brief LIN node type
 *
 * SWRD-LIN-010: Driver shall support master and slave modes
 */
typedef enum {
    LIN_NODE_MASTER,  /**< Master node - controls schedule */
    LIN_NODE_SLAVE    /**< Slave node - responds to headers */
} lin_node_type_t;

/**
 * @brief LIN message direction
 *
 * SWRD-LIN-011: Support publish/subscribe model
 */
typedef enum {
    LIN_DIR_PUBLISH,    /**< This node publishes response */
    LIN_DIR_SUBSCRIBE,  /**< This node receives response */
    LIN_DIR_IGNORE      /**< Ignore this frame ID */
} lin_direction_t;

/**
 * @brief LIN checksum type
 *
 * SWRD-LIN-012: Support classic and enhanced checksums
 */
typedef enum {
    LIN_CHECKSUM_CLASSIC,   /**< LIN 1.x - data only */
    LIN_CHECKSUM_ENHANCED   /**< LIN 2.x - PID + data */
} lin_checksum_t;

/**
 * @brief LIN frame descriptor
 *
 * LLR-LIN-010: Frame configuration structure
 */
typedef struct {
    uint8_t id;              /**< Protected ID (0-63) */
    lin_direction_t dir;     /**< Publish or subscribe */
    lin_checksum_t checksum; /**< Checksum type */
    uint8_t data_len;        /**< Data length (1-8) */
    uint8_t *data;           /**< Pointer to data buffer */
} lin_frame_t;

/**
 * @brief Schedule table entry
 *
 * SWRD-LIN-020: Schedule table support
 */
typedef struct {
    uint8_t frame_index;   /**< Index into frame table */
    uint16_t delay_ms;     /**< Delay after this frame */
} lin_schedule_entry_t;

/**
 * @brief LIN error codes
 *
 * SWRD-LIN-030: Error reporting
 */
typedef enum {
    LIN_OK = 0,
    LIN_ERR_INVALID_PARAM,
    LIN_ERR_NOT_INIT,
    LIN_ERR_TIMEOUT,
    LIN_ERR_CHECKSUM,       /**< SAF-LIN-001: Checksum mismatch */
    LIN_ERR_SYNC,           /**< SAF-LIN-002: Sync field error */
    LIN_ERR_FRAMING,        /**< SAF-LIN-003: Framing error */
    LIN_ERR_BIT,            /**< SAF-LIN-004: Bit error (bus collision) */
    LIN_ERR_NO_RESPONSE     /**< SAF-LIN-005: Slave no response */
} lin_status_t;

/**
 * @brief LIN statistics
 *
 * SWRD-LIN-031: Diagnostic counters
 */
typedef struct {
    uint32_t tx_frames;       /**< Frames transmitted */
    uint32_t rx_frames;       /**< Frames received */
    uint32_t checksum_errors; /**< Checksum failures */
    uint32_t sync_errors;     /**< Sync field errors */
    uint32_t timeout_errors;  /**< Response timeouts */
    uint32_t bus_errors;      /**< Bus collision errors */
} lin_stats_t;

/**
 * @brief LIN configuration
 *
 * SWRD-LIN-040: Initialization parameters
 */
typedef struct {
    lin_node_type_t node_type;    /**< Master or slave */
    uint32_t baud_rate;           /**< Communication speed */
    lin_frame_t *frames;          /**< Frame table */
    uint8_t frame_count;          /**< Number of frames */
    lin_schedule_entry_t *schedule; /**< Schedule table (master only) */
    uint8_t schedule_len;         /**< Schedule table length */
} lin_config_t;

/*============================================================================
 * Callback Types
 *===========================================================================*/

/**
 * @brief Frame received callback
 *
 * SWRD-LIN-050: Application notification on frame reception
 *
 * @param frame_index Index of received frame
 * @param status Reception status
 */
typedef void (*lin_rx_callback_t)(uint8_t frame_index, lin_status_t status);

/**
 * @brief Error callback
 *
 * SAF-LIN-010: Application notification on errors
 *
 * @param error Error code
 * @param frame_index Frame that caused error (-1 if N/A)
 */
typedef void (*lin_error_callback_t)(lin_status_t error, int8_t frame_index);

/*============================================================================
 * API Functions
 *===========================================================================*/

/**
 * @brief Initialize LIN driver
 *
 * SWRD-LIN-100: Initialization sequence
 *
 * LLR-LIN-100: Configure UART for LIN framing
 * LLR-LIN-101: Set up break detection
 * LLR-LIN-102: Initialize frame table
 *
 * @param channel LIN channel (0-n)
 * @param config Configuration parameters
 * @return LIN_OK on success
 */
lin_status_t lin_init(uint8_t channel, const lin_config_t *config);

/**
 * @brief Start LIN communication
 *
 * SWRD-LIN-101: Start schedule execution (master)
 * SWRD-LIN-102: Start listening for headers (slave)
 *
 * @param channel LIN channel
 * @return LIN_OK on success
 */
lin_status_t lin_start(uint8_t channel);

/**
 * @brief Stop LIN communication
 *
 * SWRD-LIN-103: Stop schedule and go idle
 *
 * @param channel LIN channel
 */
void lin_stop(uint8_t channel);

/**
 * @brief Send a single frame (master mode)
 *
 * SWRD-LIN-110: On-demand frame transmission
 *
 * @param channel LIN channel
 * @param frame_index Frame to send
 * @return LIN_OK if queued successfully
 */
lin_status_t lin_send_frame(uint8_t channel, uint8_t frame_index);

/**
 * @brief Update frame data
 *
 * SWRD-LIN-111: Application data update
 *
 * @param channel LIN channel
 * @param frame_index Frame to update
 * @param data New data
 * @param len Data length
 * @return LIN_OK on success
 */
lin_status_t lin_update_frame(uint8_t channel, uint8_t frame_index,
                              const uint8_t *data, uint8_t len);

/**
 * @brief Get frame data
 *
 * SWRD-LIN-112: Read received frame data
 *
 * @param channel LIN channel
 * @param frame_index Frame to read
 * @param data Output buffer
 * @param len Buffer length
 * @return Number of bytes copied
 */
uint8_t lin_get_frame(uint8_t channel, uint8_t frame_index,
                      uint8_t *data, uint8_t len);

/**
 * @brief Go to sleep mode
 *
 * SWRD-LIN-120: Low-power mode support
 *
 * LLR-LIN-120: Master sends sleep command
 * LLR-LIN-121: Slaves enter low-power state
 *
 * @param channel LIN channel
 */
void lin_goto_sleep(uint8_t channel);

/**
 * @brief Wake up bus
 *
 * SWRD-LIN-121: Wake-up procedure
 *
 * @param channel LIN channel
 */
void lin_wakeup(uint8_t channel);

/**
 * @brief Register callbacks
 *
 * SWRD-LIN-130: Callback registration
 *
 * @param channel LIN channel
 * @param rx_cb Frame received callback (may be NULL)
 * @param err_cb Error callback (may be NULL)
 */
void lin_register_callbacks(uint8_t channel,
                            lin_rx_callback_t rx_cb,
                            lin_error_callback_t err_cb);

/**
 * @brief Get diagnostic statistics
 *
 * SWRD-LIN-140: Diagnostic interface
 *
 * @param channel LIN channel
 * @param stats Output buffer for statistics
 */
void lin_get_stats(uint8_t channel, lin_stats_t *stats);

/**
 * @brief Reset statistics
 *
 * SWRD-LIN-141: Clear diagnostic counters
 *
 * @param channel LIN channel
 */
void lin_reset_stats(uint8_t channel);

/**
 * @brief Periodic tick handler
 *
 * LLR-LIN-130: Must be called every 1ms for timing
 *
 * @param channel LIN channel
 */
void lin_tick(uint8_t channel);

#ifdef __cplusplus
}
#endif

#endif /* LIN_DRIVER_H */
