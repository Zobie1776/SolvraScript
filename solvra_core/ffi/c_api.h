#ifndef SOLVRA_CORE_C_API_H
#define SOLVRA_CORE_C_API_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SolvraRuntime SolvraRuntime;

typedef enum {
    SOLVRA_STATUS_OK = 0,
    SOLVRA_STATUS_ERROR = 1
} SolvraStatus;

typedef struct {
    uint32_t tag;
    int64_t int_value;
    double float_value;
} SolvraValue;

SolvraRuntime* solvra_runtime_new(void);
SolvraStatus solvra_runtime_free(SolvraRuntime* runtime);
SolvraStatus solvra_runtime_execute(SolvraRuntime* runtime, const unsigned char* bytes, unsigned int len, SolvraValue* out_value);

#ifdef __cplusplus
}
#endif

#endif /* SOLVRA_CORE_C_API_H */
