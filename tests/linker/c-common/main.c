#include <stdio.h>
#include <sys/utsname.h>


void init();

//this will be emitted as COMMON
char *hello;

int main(int argc, char**argv){
    init();

    struct utsname unameData;
    uname(&unameData);
    printf(hello);
    return 42;
}
