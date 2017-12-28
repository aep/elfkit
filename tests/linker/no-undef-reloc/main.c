#include <stdio.h>

int __attribute__((weak)) never();

int main(int argc, char**argv){
    if (&never) {
        never();
    }
    printf("The quick brown fox jumps over the lazy dog");
    return 42;
}
