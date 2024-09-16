#include "dummy.h"

#define BORROW [[clang::annotate("BORROW")]]
#define MOVE [[clang::annotate("MOVE")]]

int function(BORROW const int* d1, MOVE const int* d2) {
  const int d3;
  int d4;

  d3 = *d1;

  d4 = *data(&d3, &d4);

  return 0;
}

int main() { ; }
