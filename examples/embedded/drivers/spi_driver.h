/**
 * @file spi_driver.h
 * @brief SPI Driver Interface for Sensor Communication
 *
 * @note Implements requirements from SWRD-SPI section
 */

#ifndef SPI_DRIVER_H
#define SPI_DRIVER_H

#include <stdint.h>
#include <stdbool.h>

/**
 * @defgroup SPI_Config SPI Configuration
 * @{
 */

/** SWRD-SPI-001: SPI clock shall be configurable from 100kHz to 10MHz */
#define SPI_MIN_CLOCK_HZ    100000U
#define SPI_MAX_CLOCK_HZ    10000000U

/** SWRD-SPI-002: SPI shall support all four clock modes */
typedef enum {
    SPI_MODE_0 = 0,  /**< CPOL=0, CPHA=0 */
    SPI_MODE_1 = 1,  /**< CPOL=0, CPHA=1 */
    SPI_MODE_2 = 2,  /**< CPOL=1, CPHA=0 */
    SPI_MODE_3 = 3   /**< CPOL=1, CPHA=1 */
} spi_mode_t;

/** SWRD-SPI-003: SPI shall support 8-bit and 16-bit word sizes */
typedef enum {
    SPI_WORD_8BIT  = 8,
    SPI_WORD_16BIT = 16
} spi_word_size_t;

/**
 * @brief SPI configuration structure
 *
 * SWRD-SPI-004: Configuration shall be immutable after initialization
 */
typedef struct {
    uint32_t clock_hz;       /**< Clock frequency in Hz */
    spi_mode_t mode;         /**< Clock polarity and phase */
    spi_word_size_t word;    /**< Word size */
    bool msb_first;          /**< SWRD-SPI-005: Bit order shall be configurable */
} spi_config_t;

/** @} */

/**
 * @defgroup SPI_Status SPI Status Codes
 * SWRD-SPI-006: All functions shall return status codes
 * @{
 */
typedef enum {
    SPI_OK = 0,
    SPI_ERR_INVALID_PARAM,
    SPI_ERR_BUSY,
    SPI_ERR_TIMEOUT,
    SPI_ERR_OVERRUN,
    SPI_ERR_NOT_INIT
} spi_status_t;
/** @} */

/**
 * @brief Initialize SPI peripheral
 *
 * SWRD-SPI-010: Initialization shall validate all configuration parameters
 * SWRD-SPI-011: Initialization shall be idempotent
 *
 * @param[in] channel SPI channel number (0-2)
 * @param[in] config Configuration parameters
 * @return SPI_OK on success
 */
spi_status_t spi_init(uint8_t channel, const spi_config_t *config);

/**
 * @brief Deinitialize SPI peripheral
 *
 * SWRD-SPI-012: Deinitialization shall release all hardware resources
 *
 * @param[in] channel SPI channel number
 */
void spi_deinit(uint8_t channel);

/**
 * @brief Perform full-duplex SPI transfer
 *
 * SWRD-SPI-020: Transfer shall be atomic (no interleaving with other transfers)
 * SWRD-SPI-021: Transfer shall support DMA for buffers > 16 bytes
 *
 * @param[in] channel SPI channel number
 * @param[in] tx_buf Transmit buffer (may be NULL for receive-only)
 * @param[out] rx_buf Receive buffer (may be NULL for transmit-only)
 * @param[in] len Number of bytes to transfer
 * @return SPI_OK on success
 */
spi_status_t spi_transfer(uint8_t channel,
                          const uint8_t *tx_buf,
                          uint8_t *rx_buf,
                          uint16_t len);

/**
 * @brief Asynchronous SPI transfer with callback
 *
 * SWRD-SPI-022: Async transfer shall not block caller
 * SWRD-SPI-023: Callback shall be invoked from ISR context
 *
 * @param[in] channel SPI channel number
 * @param[in] tx_buf Transmit buffer
 * @param[out] rx_buf Receive buffer
 * @param[in] len Number of bytes
 * @param[in] callback Completion callback
 * @param[in] user_data User context for callback
 * @return SPI_OK if transfer started
 */
typedef void (*spi_callback_t)(spi_status_t status, void *user_data);

spi_status_t spi_transfer_async(uint8_t channel,
                                const uint8_t *tx_buf,
                                uint8_t *rx_buf,
                                uint16_t len,
                                spi_callback_t callback,
                                void *user_data);

/**
 * @brief Check if SPI channel is busy
 *
 * SWRD-SPI-024: Status query shall be non-blocking
 *
 * @param[in] channel SPI channel number
 * @return true if transfer in progress
 */
bool spi_is_busy(uint8_t channel);

/**
 * @brief Abort ongoing SPI transfer
 *
 * SWRD-SPI-025: Abort shall complete within 1ms
 * SAF-SPI-001: Abort shall leave hardware in known state
 *
 * @param[in] channel SPI channel number
 */
void spi_abort(uint8_t channel);

#endif /* SPI_DRIVER_H */
