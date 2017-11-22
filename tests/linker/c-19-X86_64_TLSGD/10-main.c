#include <stdio.h>
#include <pthread.h>
#include <string.h>

__thread char x[] = "The quick brown fox jumps over the lazy dog";


// this will not affect x, because tr gets a thread local copy of x
void *tr(void*_) {
    memcpy(&x, "nope\0", 5);
}

int main(int argc, char**argv) {
    pthread_t t;
    pthread_create(&t, 0, &tr, 0);
    pthread_join(t, 0);

    printf(x);
    return 42;
}
