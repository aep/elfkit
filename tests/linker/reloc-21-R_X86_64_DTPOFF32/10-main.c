#include <stdio.h>
#include <pthread.h>
#include <string.h>


__thread char derp[] = "this is just some random crap to test alingment";

typedef struct {
    char x1[32];
    char x2[32];
    char x3[32];
} blarp;

__thread blarp bla;
__thread char ferp[] = "this is just some random crap to test alingment";


// this will not affect x, because tr gets a thread local copy of x
void *tr(void*_) {
    memcpy(&bla.x1, "nope\0", 5);
    memcpy(&bla.x2, "nope\0", 5);
}

int main(int argc, char**argv) {
    blarp *b = &bla;
    strcpy(b->x1, "The quick brown ");
    strcpy(b->x2, "fox jumps over the ");
    strcpy(b->x3, "lazy dog");

    pthread_t t;
    pthread_create(&t, 0, &tr, 0);
    pthread_join(t, 0);

    printf("%s%s%s", bla.x1,bla.x2,bla.x3);
    return 42;
}

