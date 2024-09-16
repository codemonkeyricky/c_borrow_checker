#include "dummy.h"

#define BORROW [[clang::annotate("BORROW")]]
#define MOVE [[clang::annotate("MOVE")]]

int* single(MOVE int* d) { return d; }

int function(BORROW const int* d1, MOVE const int* d2) {
  int d3;
  int d4;
  const int d5;

  d3 = *d1;

  d4 = *data(&d3, &d4);

  int *d6 = single(single(single(d6)));

  return d3;
}

int main() { ; }
