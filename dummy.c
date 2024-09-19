
#include "dummy.h"
#define MOVE __attribute__((annotate("MOVE")))
#define BORROW __attribute__((annotate("BORROW")))
#define EMPTY __attribute__((annotate("EMPTY")))

int* single(MOVE int* d) { return d; }

int function(MOVE const int* d1, BORROW const int* d2) {
  EMPTY int* clear = 0;
  int* d3;
  int d4;

  d1 = clear;

  int * a = single(&d1);

  d3 = d1;
  *d3 = *d1;
}