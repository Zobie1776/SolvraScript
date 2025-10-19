#include "../ffi/c_api.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    NovaRuntime* runtime = nova_runtime_new();
    unsigned char program[] = { 'N', 'V', 'B', 'C' }; /* placeholder header */
    NovaValue value = {0};
    NovaStatus status = nova_runtime_execute(runtime, program, sizeof(program), &value);
    if (status == NOVA_STATUS_OK) {
        printf("Execution succeeded with tag %u\n", value.tag);
    } else {
        printf("Execution failed\n");
    }
    nova_runtime_free(runtime);
    return 0;
}
