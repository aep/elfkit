#include <stdio.h>
#include <pthread.h>
#include <string.h>

__thread char bla[] = "this is just some random crap to test alingment and to make an incorrect offset 0 fail";
__thread char x[] = "The quick brown fox jumps over the lazy dog";
__thread char bla2[] = "more crap at the end of tdata";

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
