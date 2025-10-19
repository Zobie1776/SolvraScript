#ifndef NOVA_CORE_C_API_H
#define NOVA_CORE_C_API_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NovaRuntime NovaRuntime;

typedef enum {
    NOVA_STATUS_OK = 0,
    NOVA_STATUS_ERROR = 1
} NovaStatus;

typedef struct {
    uint32_t tag;
    int64_t int_value;
    double float_value;
} NovaValue;

NovaRuntime* nova_runtime_new(void);
NovaStatus nova_runtime_free(NovaRuntime* runtime);
NovaStatus nova_runtime_execute(NovaRuntime* runtime, const unsigned char* bytes, unsigned int len, NovaValue* out_value);

#ifdef __cplusplus
}
#endif

#endif /* NOVA_CORE_C_API_H */
