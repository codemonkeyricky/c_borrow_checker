

// struct sDATA* function(/* MOVE */ int* in, /* BORROW */ int* in_out) {
//   struct sDATA* d = malloc(sizeof(struct sDATA));
//   return d;
// }
#define MY_ATTR [[clang::annotate("XXX")]]
#define BORROW [[clang::annotate("BORROW")]]
#define MOVE [[clang::annotate("MOVE")]]

/* MOVE BORROW */
MY_ATTR int function(BORROW int d1, MOVE int d2) {
  int d3;
  int d4;

  d3 = d1;
}

int main() { ; }
