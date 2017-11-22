#include <stdio.h>

char *x = 0;
__attribute__((constructor)) void xinit() {
    x = "The quick brown fox jumps over the lazy dog";
}

__attribute__((destructor)) void xfini() {
    printf(x);
}

int main(int argc, char**argv) {
    return 42;
}
