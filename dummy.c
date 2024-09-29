
// #include <time.h>
__attribute__((annotate("OWNERSHIP_DROP"))) void inline ownership_drop(
    void *p) {}
#define MOVE __attribute__((annotate("MOVE")))
#define BORROW __attribute__((annotate("BORROW")))
#define OWNERSHIP_DROP(p) (ownership_drop(p))

struct Subet {
  MOVE void *a;
  MOVE void *b;
  MOVE void *c;
};

struct Set {
  MOVE void *a;
  MOVE struct Subset *s;
};

// struct dummy {
//   MOVE int *d0;
//   BORROW int d1;
// };

// int* single(MOVE int* d) { return d; }
MOVE int *double_param(MOVE int *a, MOVE int *b);

MOVE int function(MOVE const int *d1, BORROW const int *d2) {
  int *a, *b;

  // OWNERSHIP_DROP(a);
  volatile int dummy = 0;
  volatile int dummy2 = 0;

  if (dummy)
    return 0;

  if (dummy) {
    // a = double_param(a, b);
    a = double_param(double_param(a, b), b);
  } else if (dummy2) {
    b = double_param(a, a);
  }

  // d3 = d1;
  // *d3 = *d1;
}