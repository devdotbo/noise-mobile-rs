/* Generated C bindings for noise-mobile-rust
 * Copyright 2025 PermissionlessTech Contributors
 * SPDX-License-Identifier: MIT OR Apache-2.0
 *
 * This file is auto-generated by cbindgen.
 * DO NOT EDIT THIS FILE MANUALLY.
 */

#include <stdint.h>
#include <stddef.h>


#ifndef NOISE_MOBILE_H
#define NOISE_MOBILE_H

#pragma once

/* Generated with cbindgen:0.29.0 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Maximum message length supported by Noise
 */
#define NoiseSession_MAX_MESSAGE_LEN 65535

#define NOISE_MAX_MESSAGE_LEN 65535

#define NOISE_MAX_PAYLOAD_LEN (65535 - 16)

#define NOISE_TAG_LEN 16

/**
 * FFI-safe error codes returned by C API functions
 */
typedef enum NoiseErrorCode {
  /**
   * Operation completed successfully
   */
  SUCCESS = 0,
  /**
   * Invalid parameter provided
   */
  INVALID_PARAMETER = 1,
  /**
   * Out of memory
   */
  OUT_OF_MEMORY = 2,
  /**
   * Handshake failed
   */
  HANDSHAKE_FAILED = 3,
  /**
   * Encryption operation failed
   */
  ENCRYPTION_FAILED = 4,
  /**
   * Decryption operation failed
   */
  DECRYPTION_FAILED = 5,
  /**
   * Provided buffer is too small
   */
  BUFFER_TOO_SMALL = 6,
  /**
   * Operation invalid in current state
   */
  INVALID_STATE = 7,
  /**
   * General protocol error
   */
  PROTOCOL_ERROR = 8,
} NoiseErrorCode;

/**
 * FFI-safe session mode
 */
typedef enum NoiseMode {
  /**
   * Session acts as initiator (client)
   */
  INITIATOR = 0,
  /**
   * Session acts as responder (server)
   */
  RESPONDER = 1,
} NoiseMode;

typedef struct NoiseError NoiseError;

/**
 * Opaque pointer type for Noise sessions
 */
typedef struct NoiseSessionFFI {
  uint8_t _private[0];
} NoiseSessionFFI;

/**
 * Create a new Noise session
 */
 struct NoiseSessionFFI *noise_session_new(int mode, int *error);

/**
 * Create a new Noise session with a specific private key
 */

struct NoiseSessionFFI *noise_session_new_with_key(const unsigned char *private_key,
                                                   size_t private_key_len,
                                                   int mode,
                                                   int *error);

/**
 * Free a Noise session
 */
 void noise_session_free(struct NoiseSessionFFI *session);

/**
 * Write a handshake message
 */

int noise_write_message(struct NoiseSessionFFI *session,
                        const unsigned char *payload,
                        size_t payload_len,
                        unsigned char *output,
                        size_t *output_len);

/**
 * Read a handshake message
 */

int noise_read_message(struct NoiseSessionFFI *session,
                       const unsigned char *input,
                       size_t input_len,
                       unsigned char *payload,
                       size_t *payload_len);

/**
 * Check if handshake is complete
 */
 int noise_is_handshake_complete(struct NoiseSessionFFI *session);

/**
 * Encrypt a message
 */

int noise_encrypt(struct NoiseSessionFFI *session,
                  const unsigned char *plaintext,
                  size_t plaintext_len,
                  unsigned char *ciphertext,
                  size_t *ciphertext_len);

/**
 * Decrypt a message
 */

int noise_decrypt(struct NoiseSessionFFI *session,
                  const unsigned char *ciphertext,
                  size_t ciphertext_len,
                  unsigned char *plaintext,
                  size_t *plaintext_len);

/**
 * Get the remote peer's static public key
 */

int noise_get_remote_static(struct NoiseSessionFFI *session,
                            unsigned char *output,
                            size_t *output_len);

/**
 * Get the maximum message length
 */
 size_t noise_max_message_len(void);

/**
 * Get the maximum payload length
 */
 size_t noise_max_payload_len(void);

/**
 * Get error string for an error code
 */
 const char *noise_error_string(int error);

#endif  /* NOISE_MOBILE_H */
