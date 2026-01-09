/**
 * @file ecu_monitor.hpp
 * @brief ECU Health Monitor for Automotive Safety Systems
 *
 * Implements continuous monitoring of ECU health parameters including
 * voltage, temperature, clock integrity, and memory tests.
 *
 * @par Safety Classification
 * ISO 26262 ASIL-D compliant
 *
 * @par Requirements Document
 * REQ-300: ECU monitor shall detect hardware failures within 10ms
 * REQ-301: ECU monitor shall support graceful degradation
 * REQ-302: ECU monitor shall log all detected faults
 */

#ifndef ECU_MONITOR_HPP
#define ECU_MONITOR_HPP

#include <cstdint>
#include <array>
#include <functional>

namespace automotive {
namespace safety {

/**
 * @brief Voltage monitoring thresholds
 *
 * REQ-310: Supply voltage shall be monitored continuously
 */
struct VoltageThresholds {
    uint16_t undervolt_mv;    ///< REQ-311: Undervoltage threshold
    uint16_t overvolt_mv;     ///< REQ-312: Overvoltage threshold
    uint16_t nominal_mv;      ///< REQ-313: Nominal voltage
    uint16_t hysteresis_mv;   ///< REQ-314: Threshold hysteresis
};

/**
 * @brief Default voltage thresholds for 12V automotive
 *
 * REQ-315: Default thresholds per ISO 16750
 */
constexpr VoltageThresholds DEFAULT_12V_THRESHOLDS = {
    .undervolt_mv = 9000,     // 9V minimum
    .overvolt_mv = 16000,     // 16V maximum
    .nominal_mv = 13800,      // 13.8V nominal
    .hysteresis_mv = 500      // 500mV hysteresis
};

/**
 * @brief Temperature monitoring thresholds
 *
 * REQ-320: Junction temperature shall be monitored
 */
struct TemperatureThresholds {
    int16_t warning_high_c;   ///< REQ-321: High temperature warning
    int16_t shutdown_high_c;  ///< REQ-322: High temperature shutdown
    int16_t warning_low_c;    ///< REQ-323: Low temperature warning
    int16_t shutdown_low_c;   ///< REQ-324: Low temperature shutdown
};

/**
 * @brief Default temperature thresholds
 *
 * REQ-325: Default thresholds per AEC-Q100 Grade 1
 */
constexpr TemperatureThresholds DEFAULT_TEMP_THRESHOLDS = {
    .warning_high_c = 125,    // 125C warning
    .shutdown_high_c = 150,   // 150C shutdown
    .warning_low_c = -40,     // -40C warning
    .shutdown_low_c = -50     // -50C shutdown
};

/**
 * @brief Fault codes for ECU monitor
 *
 * REQ-330: All faults shall have unique codes
 */
enum class FaultCode : uint16_t {
    NoFault = 0x0000,

    // Voltage faults (0x01xx)
    Undervoltage = 0x0100,         ///< REQ-331: Supply undervoltage
    Overvoltage = 0x0101,          ///< REQ-332: Supply overvoltage
    VoltageUnstable = 0x0102,      ///< REQ-333: Voltage instability

    // Temperature faults (0x02xx)
    OvertemperatureWarn = 0x0200,  ///< REQ-334: High temp warning
    OvertemperatureShut = 0x0201,  ///< REQ-335: High temp shutdown
    UndertemperatureWarn = 0x0202, ///< REQ-336: Low temp warning
    UndertemperatureShut = 0x0203, ///< REQ-337: Low temp shutdown

    // Clock faults (0x03xx)
    ClockDrift = 0x0300,           ///< REQ-338: Clock frequency drift
    ClockLost = 0x0301,            ///< REQ-339: Clock signal lost

    // Memory faults (0x04xx)
    RamError = 0x0400,             ///< REQ-340: RAM test failure
    FlashError = 0x0401,           ///< REQ-341: Flash checksum error
    StackOverflow = 0x0402,        ///< REQ-342: Stack overflow detected

    // Watchdog faults (0x05xx)
    WatchdogReset = 0x0500,        ///< REQ-343: Watchdog reset occurred
    WatchdogTimeout = 0x0501,      ///< REQ-344: Watchdog timeout

    // Communication faults (0x06xx)
    CanBusOff = 0x0600,            ///< REQ-345: CAN bus-off state
    CanErrorPassive = 0x0601,      ///< REQ-346: CAN error passive
    LinNoResponse = 0x0602         ///< REQ-347: LIN slave no response
};

/**
 * @brief Fault severity levels
 *
 * REQ-350: Faults shall be classified by severity
 */
enum class FaultSeverity : uint8_t {
    Info = 0,       ///< Informational only
    Warning = 1,    ///< REQ-351: Warning - continue with caution
    Error = 2,      ///< REQ-352: Error - degraded operation
    Critical = 3    ///< REQ-353: Critical - immediate safe state
};

/**
 * @brief Fault record structure
 *
 * REQ-302: Fault logging structure
 */
struct FaultRecord {
    uint32_t timestamp_ms;    ///< REQ-354: Time of fault detection
    FaultCode code;           ///< REQ-355: Fault code
    FaultSeverity severity;   ///< REQ-356: Fault severity
    uint16_t data;            ///< REQ-357: Fault-specific data
};

/**
 * @brief Fault callback function type
 *
 * REQ-360: Application notification callback
 */
using FaultCallback = std::function<void(const FaultRecord&)>;

/**
 * @brief ECU monitor configuration
 *
 * REQ-370: Monitor configuration structure
 */
struct MonitorConfig {
    VoltageThresholds voltage;       ///< Voltage thresholds
    TemperatureThresholds temp;      ///< Temperature thresholds
    uint32_t check_interval_ms;      ///< REQ-371: Check interval
    uint8_t ram_test_pattern;        ///< REQ-372: RAM test pattern
    bool enable_clock_monitor;       ///< REQ-373: Clock monitoring enable
    FaultCallback fault_callback;    ///< REQ-374: Fault notification
};

/**
 * @brief Default monitor configuration
 *
 * REQ-375: Conservative defaults
 */
constexpr uint32_t DEFAULT_CHECK_INTERVAL_MS = 10;  // REQ-300: 10ms detection

/**
 * @class EcuMonitor
 * @brief Main ECU health monitoring class
 *
 * REQ-380: Single monitor instance per ECU
 */
class EcuMonitor {
public:
    /**
     * @brief Maximum fault log entries
     *
     * REQ-381: Fault log shall store at least 64 entries
     */
    static constexpr size_t MAX_FAULT_LOG = 64;

    /**
     * @brief Construct monitor with configuration
     *
     * REQ-382: Configuration applied at construction
     *
     * @param config Monitor configuration
     */
    explicit EcuMonitor(const MonitorConfig& config);

    /**
     * @brief Start monitoring
     *
     * REQ-383: Start shall begin periodic checks
     *
     * @return true if started successfully
     */
    bool start();

    /**
     * @brief Stop monitoring
     *
     * REQ-384: Stop shall halt all checks
     */
    void stop();

    /**
     * @brief Periodic monitoring tick
     *
     * REQ-385: Must be called at configured interval
     * REQ-386: Shall complete within 1ms
     */
    void tick();

    /**
     * @brief Force immediate check
     *
     * REQ-387: On-demand health check
     *
     * @return true if all checks pass
     */
    bool check_now();

    /**
     * @brief Get current voltage reading
     *
     * REQ-390: Voltage shall be readable
     *
     * @return Voltage in millivolts
     */
    uint16_t get_voltage_mv() const;

    /**
     * @brief Get current temperature reading
     *
     * REQ-391: Temperature shall be readable
     *
     * @return Temperature in Celsius
     */
    int16_t get_temperature_c() const;

    /**
     * @brief Check if voltage is in valid range
     *
     * REQ-392: Voltage status query
     */
    bool is_voltage_ok() const;

    /**
     * @brief Check if temperature is in valid range
     *
     * REQ-393: Temperature status query
     */
    bool is_temperature_ok() const;

    /**
     * @brief Get active fault count
     *
     * REQ-394: Active fault count
     */
    size_t get_active_fault_count() const;

    /**
     * @brief Get fault log
     *
     * REQ-395: Access to fault history
     *
     * @param buffer Output buffer for fault records
     * @param max_count Maximum records to retrieve
     * @return Number of records copied
     */
    size_t get_fault_log(FaultRecord* buffer, size_t max_count) const;

    /**
     * @brief Clear fault log
     *
     * REQ-396: Fault log clear function
     */
    void clear_fault_log();

    /**
     * @brief Acknowledge active faults
     *
     * REQ-397: Fault acknowledgment
     *
     * @param code Fault code to acknowledge (NoFault = all)
     */
    void acknowledge_fault(FaultCode code);

private:
    MonitorConfig config_;
    bool running_;

    // REQ-400: State variables
    uint16_t current_voltage_mv_;
    int16_t current_temperature_c_;
    uint32_t last_check_ms_;

    // REQ-401: Fault log storage
    std::array<FaultRecord, MAX_FAULT_LOG> fault_log_;
    size_t fault_log_head_;
    size_t fault_log_count_;
    uint32_t active_faults_;  // Bitmap

    // Private methods

    /**
     * @brief Check supply voltage
     *
     * REQ-410: Voltage check implementation
     */
    void check_voltage();

    /**
     * @brief Check temperature
     *
     * REQ-411: Temperature check implementation
     */
    void check_temperature();

    /**
     * @brief Check clock integrity
     *
     * REQ-412: Clock check implementation
     */
    void check_clock();

    /**
     * @brief Run RAM test
     *
     * REQ-413: RAM test implementation
     */
    void check_ram();

    /**
     * @brief Log a fault
     *
     * REQ-414: Fault logging implementation
     *
     * @param code Fault code
     * @param severity Fault severity
     * @param data Additional data
     */
    void log_fault(FaultCode code, FaultSeverity severity, uint16_t data);

    /**
     * @brief Read ADC for voltage
     *
     * REQ-415: Hardware abstraction
     */
    uint16_t read_voltage_adc();

    /**
     * @brief Read temperature sensor
     *
     * REQ-416: Hardware abstraction
     */
    int16_t read_temperature_sensor();

    /**
     * @brief Get current timestamp
     *
     * REQ-417: Timestamp source
     */
    uint32_t get_timestamp_ms();
};

}  // namespace safety
}  // namespace automotive

#endif  // ECU_MONITOR_HPP
