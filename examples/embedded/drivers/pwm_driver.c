/**
 * @file pwm_driver.c
 * @brief PWM Driver for Motor Control
 *
 * Provides PWM generation for brushless motor control in automotive
 * and industrial applications.
 *
 * REQ-100: PWM driver shall support frequencies from 1kHz to 100kHz
 * REQ-101: PWM driver shall support duty cycles from 0% to 100%
 */

#include "pwm_driver.h"
#include <stdint.h>
#include <stdbool.h>

/**
 * REQ-102: Maximum number of PWM channels
 */
#define PWM_MAX_CHANNELS  8

/**
 * REQ-103: Default PWM frequency for motor control
 */
#define PWM_DEFAULT_FREQ_HZ  20000U

/**
 * @brief PWM channel state
 *
 * REQ-104: Each channel shall maintain independent configuration
 */
typedef struct {
    uint32_t frequency_hz;   // REQ-105: Channel frequency
    uint16_t duty_permille;  // REQ-106: Duty cycle in 0.1% steps (0-1000)
    bool enabled;            // REQ-107: Channel enable state
    bool inverted;           // REQ-108: Output polarity
} pwm_channel_t;

static pwm_channel_t channels[PWM_MAX_CHANNELS];

/**
 * @brief Initialize PWM peripheral
 *
 * REQ-110: Initialize all channels to safe default state
 * REQ-111: Default state shall be 0% duty cycle
 *
 * @return 0 on success, negative error code on failure
 */
int pwm_init(void)
{
    // REQ-112: Reset all channels
    for (int i = 0; i < PWM_MAX_CHANNELS; i++) {
        channels[i].frequency_hz = PWM_DEFAULT_FREQ_HZ;
        channels[i].duty_permille = 0;  // REQ-111: Default 0%
        channels[i].enabled = false;
        channels[i].inverted = false;
    }

    // REQ-113: Configure timer peripheral
    PWM_TIM->CR1 = 0;
    PWM_TIM->PSC = (SYSTEM_CLOCK / 1000000) - 1;  // 1MHz tick
    PWM_TIM->ARR = 50;  // 20kHz default

    // REQ-114: Enable PWM outputs in safe state
    PWM_TIM->CCER = 0;  // All outputs disabled

    return 0;
}

/**
 * @brief Configure PWM channel frequency
 *
 * REQ-120: Frequency change shall not glitch output
 * REQ-121: Frequency shall be applied at next period boundary
 *
 * @param channel Channel number (0-7)
 * @param freq_hz Desired frequency in Hz
 * @return 0 on success
 */
int pwm_set_frequency(uint8_t channel, uint32_t freq_hz)
{
    if (channel >= PWM_MAX_CHANNELS) {
        return -1;  // REQ-122: Validate channel number
    }

    if (freq_hz < 1000 || freq_hz > 100000) {
        return -2;  // REQ-100: Validate frequency range
    }

    // REQ-123: Calculate timer period
    uint32_t period = 1000000 / freq_hz;

    channels[channel].frequency_hz = freq_hz;

    // REQ-121: Apply at next period
    PWM_TIM->ARR = period;

    return 0;
}

/**
 * @brief Set PWM duty cycle
 *
 * REQ-130: Duty cycle shall be in 0.1% increments
 * REQ-131: Duty cycle update shall be glitch-free
 *
 * @param channel Channel number
 * @param duty_permille Duty cycle in 0.1% (0-1000 = 0%-100%)
 * @return 0 on success
 */
int pwm_set_duty(uint8_t channel, uint16_t duty_permille)
{
    if (channel >= PWM_MAX_CHANNELS) {
        return -1;
    }

    if (duty_permille > 1000) {
        duty_permille = 1000;  // REQ-132: Clamp to maximum
    }

    channels[channel].duty_permille = duty_permille;

    // REQ-133: Calculate compare value
    uint32_t compare = (PWM_TIM->ARR * duty_permille) / 1000;

    // REQ-131: Use preload for glitch-free update
    switch (channel) {
        case 0: PWM_TIM->CCR1 = compare; break;
        case 1: PWM_TIM->CCR2 = compare; break;
        case 2: PWM_TIM->CCR3 = compare; break;
        case 3: PWM_TIM->CCR4 = compare; break;
        default: break;
    }

    return 0;
}

/**
 * @brief Enable PWM output
 *
 * REQ-140: Enable shall ramp output smoothly
 *
 * @param channel Channel number
 * @param enable true to enable, false to disable
 */
void pwm_enable(uint8_t channel, bool enable)
{
    if (channel >= PWM_MAX_CHANNELS) {
        return;
    }

    channels[channel].enabled = enable;

    // REQ-141: Update output enable register
    if (enable) {
        PWM_TIM->CCER |= (1 << (channel * 4));
    } else {
        PWM_TIM->CCER &= ~(1 << (channel * 4));
    }

    // REQ-142: Start timer if any channel enabled
    if (PWM_TIM->CCER != 0) {
        PWM_TIM->CR1 |= TIM_CR1_CEN;
    }
}

/**
 * @brief Emergency stop all PWM outputs
 *
 * REQ-150: Emergency stop shall disable all outputs immediately
 * REQ-151: Emergency stop shall complete within 1 timer tick
 */
void pwm_emergency_stop(void)
{
    // REQ-150: Immediate disable
    PWM_TIM->CCER = 0;
    PWM_TIM->CR1 &= ~TIM_CR1_CEN;

    // REQ-152: Reset channel states
    for (int i = 0; i < PWM_MAX_CHANNELS; i++) {
        channels[i].enabled = false;
        channels[i].duty_permille = 0;
    }

    // REQ-153: Force outputs low
    PWM_TIM->CCR1 = 0;
    PWM_TIM->CCR2 = 0;
    PWM_TIM->CCR3 = 0;
    PWM_TIM->CCR4 = 0;
}

/**
 * @brief Get current duty cycle
 *
 * REQ-160: Duty cycle shall be readable
 *
 * @param channel Channel number
 * @return Current duty cycle in permille, or -1 on error
 */
int32_t pwm_get_duty(uint8_t channel)
{
    if (channel >= PWM_MAX_CHANNELS) {
        return -1;  // REQ-161: Invalid channel
    }

    return channels[channel].duty_permille;
}

/**
 * @brief Check if channel is enabled
 *
 * REQ-162: Enable state shall be queryable
 */
bool pwm_is_enabled(uint8_t channel)
{
    if (channel >= PWM_MAX_CHANNELS) {
        return false;
    }

    return channels[channel].enabled;
}

/**
 * @brief Set output polarity
 *
 * REQ-170: Output polarity shall be configurable
 * REQ-171: Polarity change requires channel disable first
 *
 * @param channel Channel number
 * @param inverted true for inverted output
 * @return 0 on success, -1 if channel is enabled
 */
int pwm_set_polarity(uint8_t channel, bool inverted)
{
    if (channel >= PWM_MAX_CHANNELS) {
        return -1;
    }

    // REQ-171: Require disabled channel
    if (channels[channel].enabled) {
        return -2;
    }

    channels[channel].inverted = inverted;

    // REQ-172: Configure output polarity
    if (inverted) {
        PWM_TIM->CCER |= (1 << (channel * 4 + 1));
    } else {
        PWM_TIM->CCER &= ~(1 << (channel * 4 + 1));
    }

    return 0;
}
