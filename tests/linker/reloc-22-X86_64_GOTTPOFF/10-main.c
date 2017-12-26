#include <stdio.h>
#include <pthread.h>
#include <string.h>


__thread char x1[] = "The quick brown ";
__thread char bla[] = "this is just some random crap to test alingment";
__thread char x2[] = "jumps over the ";
__thread char bla4[] = "this is just some random crap to test alingment";
__thread char bla3[] = "this is just some random crap to test alingment";
__thread char x3[] = "lazy dog";
__thread char bla2[] = "this is just some random crap to test alingment";


// this will not affect x, because tr gets a thread local copy of x
void *tr(void*_) {
    memcpy(&x1, "nope\0", 5);
    memcpy(&x2, "nope\0", 5);
}

int main(int argc, char**argv) {
    pthread_t t;
    pthread_create(&t, 0, &tr, 0);
    pthread_join(t, 0);

    printf("%s%s%s", x1,x2,x3);
    return 42;
}
