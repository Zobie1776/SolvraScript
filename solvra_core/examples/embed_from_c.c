#include "../ffi/c_api.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    SolvraRuntime* runtime = solvra_runtime_new();
    unsigned char program[] = { 'N', 'V', 'B', 'C' }; /* placeholder header */
    SolvraValue value = {0};
    SolvraStatus status = solvra_runtime_execute(runtime, program, sizeof(program), &value);
    if (status == SOLVRA_STATUS_OK) {
        printf("Execution succeeded with tag %u\n", value.tag);
    } else {
        printf("Execution failed\n");
    }
    solvra_runtime_free(runtime);
    return 0;
}
