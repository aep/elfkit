#include <stdio.h>
#include <sys/utsname.h>

int main(int argc, char**argv){
    struct utsname unameData;
    uname(&unameData);
    printf("The quick brown fox jumps over the lazy dog");
    return 42;
}
