/**
 * @file watchdog.cpp
 * @brief Independent Watchdog Monitor for Safety-Critical Systems
 *
 * This module implements an independent watchdog that monitors system health
 * and triggers safe shutdown if software becomes unresponsive.
 *
 * @par Safety Classification
 * ASIL-D (Automotive Safety Integrity Level D)
 *
 * @par Related Requirements
 * - SYS-SAF-100: System shall detect software hangs within 100ms
 * - SYS-SAF-101: System shall enter safe state upon watchdog timeout
 */

#include "watchdog.hpp"
#include "hw_wdt.h"
#include "safe_state.hpp"
#include "diagnostic_log.hpp"

namespace safety {

/**
 * @brief Watchdog timeout period in milliseconds
 *
 * SAF-WDT-001: Watchdog timeout shall be configurable between 10ms and 1000ms
 * SAF-WDT-002: Default timeout shall be 100ms (ASIL-D requirement)
 */
constexpr uint32_t WDT_DEFAULT_TIMEOUT_MS = 100U;

/**
 * @brief Maximum allowed deviation from kick interval
 *
 * SAF-WDT-003: Watchdog shall detect timing violations exceeding 10%
 */
constexpr uint32_t WDT_TIMING_TOLERANCE_PERCENT = 10U;

/**
 * @class Watchdog
 * @brief Hardware watchdog wrapper with timing monitoring
 *
 * SAF-WDT-010: Watchdog class shall be non-copyable
 * SAF-WDT-011: Watchdog shall maintain kick history for diagnostics
 */
class Watchdog {
public:
    /**
     * @brief Construct watchdog with specified timeout
     *
     * SAF-WDT-020: Constructor shall not start watchdog automatically
     *
     * @param timeout_ms Timeout period in milliseconds
     */
    explicit Watchdog(uint32_t timeout_ms = WDT_DEFAULT_TIMEOUT_MS)
        : m_timeout_ms(timeout_ms)
        , m_started(false)
        , m_last_kick_time(0)
        , m_kick_count(0)
        , m_late_kick_count(0)
    {
        // SAF-WDT-021: Validate timeout range at construction
        if (timeout_ms < 10 || timeout_ms > 1000) {
            DiagLog::error(DiagCode::WDT_INVALID_TIMEOUT, timeout_ms);
            m_timeout_ms = WDT_DEFAULT_TIMEOUT_MS;
        }
    }

    // SAF-WDT-010: Non-copyable
    Watchdog(const Watchdog&) = delete;
    Watchdog& operator=(const Watchdog&) = delete;

    /**
     * @brief Start the hardware watchdog
     *
     * SAF-WDT-030: Start shall configure hardware before enabling
     * SAF-WDT-031: Start shall be callable only once
     *
     * @return true if started successfully
     */
    bool start() {
        if (m_started) {
            // SAF-WDT-032: Log duplicate start attempts
            DiagLog::warn(DiagCode::WDT_ALREADY_STARTED);
            return false;
        }

        // SAF-WDT-033: Configure hardware watchdog
        HW_WDT->TIMEOUT = ms_to_ticks(m_timeout_ms);
        HW_WDT->CTRL = WDT_CTRL_ENABLE | WDT_CTRL_RESET_ON_TIMEOUT;

        m_started = true;
        m_last_kick_time = get_system_time_ms();

        // SAF-WDT-034: Log watchdog activation
        DiagLog::info(DiagCode::WDT_STARTED, m_timeout_ms);

        return true;
    }

    /**
     * @brief Kick (refresh) the watchdog
     *
     * SAF-WDT-040: Kick shall reset hardware counter
     * SAF-WDT-041: Kick shall verify timing constraints
     *
     * @return true if kick was within timing tolerance
     */
    bool kick() {
        if (!m_started) {
            return false;  // SAF-WDT-042: Reject kick if not started
        }

        uint32_t now = get_system_time_ms();
        uint32_t elapsed = now - m_last_kick_time;

        // SAF-WDT-043: Check for late kicks
        uint32_t expected_interval = m_timeout_ms / 2;  // Kick at 50% of timeout
        uint32_t tolerance = (expected_interval * WDT_TIMING_TOLERANCE_PERCENT) / 100;

        bool timing_ok = true;
        if (elapsed > expected_interval + tolerance) {
            m_late_kick_count++;
            timing_ok = false;
            // SAF-WDT-044: Log timing violations
            DiagLog::warn(DiagCode::WDT_LATE_KICK, elapsed, expected_interval);
        }

        // SAF-WDT-045: Always kick hardware even if late
        HW_WDT->KICK = WDT_KICK_KEY;

        m_last_kick_time = now;
        m_kick_count++;

        return timing_ok;
    }

    /**
     * @brief Get diagnostic statistics
     *
     * SAF-WDT-050: Statistics shall be available for diagnostics
     */
    struct Stats {
        uint32_t kick_count;
        uint32_t late_kick_count;
        uint32_t timeout_ms;
        bool is_running;
    };

    Stats get_stats() const {
        return {
            m_kick_count,
            m_late_kick_count,
            m_timeout_ms,
            m_started
        };
    }

private:
    uint32_t m_timeout_ms;
    bool m_started;
    uint32_t m_last_kick_time;
    uint32_t m_kick_count;
    uint32_t m_late_kick_count;

    static uint32_t ms_to_ticks(uint32_t ms) {
        return (ms * WDT_CLOCK_HZ) / 1000U;
    }
};

/**
 * @brief Global watchdog instance
 *
 * SAF-WDT-060: Single watchdog instance shall be used system-wide
 */
static Watchdog g_watchdog;

/**
 * @brief Initialize and start system watchdog
 *
 * HLR-SAF-001: Safety monitor shall be initialized before main loop
 *
 * @param timeout_ms Watchdog timeout (0 = use default)
 */
void watchdog_init(uint32_t timeout_ms) {
    if (timeout_ms == 0) {
        timeout_ms = WDT_DEFAULT_TIMEOUT_MS;
    }

    g_watchdog = Watchdog(timeout_ms);

    // HLR-SAF-002: Log initialization parameters
    DiagLog::info(DiagCode::WDT_INIT, timeout_ms);
}

/**
 * @brief Start the watchdog
 *
 * HLR-SAF-003: Watchdog start shall be explicit action
 */
bool watchdog_start() {
    return g_watchdog.start();
}

/**
 * @brief Periodic watchdog service function
 *
 * HLR-SAF-004: Application shall call this from main loop
 * HLR-SAF-005: Call frequency shall be at least 2x timeout period
 */
void watchdog_kick() {
    g_watchdog.kick();
}

/**
 * @brief Watchdog timeout interrupt handler
 *
 * SAF-WDT-070: Timeout handler shall trigger safe state
 * SAF-WDT-071: Handler shall log diagnostic data before reset
 *
 * @note This runs in NMI context - very limited operations allowed
 */
extern "C" void WDT_IRQHandler(void) {
    // SAF-WDT-072: Capture diagnostic snapshot
    DiagLog::emergency(DiagCode::WDT_TIMEOUT);

    // SAF-WDT-073: Trigger safe state
    enter_safe_state(SafeStateReason::WatchdogTimeout);

    // SAF-WDT-074: Hardware reset will occur after this
}

} // namespace safety
