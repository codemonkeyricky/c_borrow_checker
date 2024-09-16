
#define BORROW [[clang::annotate("BORROW")]]
#define MOVE [[clang::annotate("MOVE")]]

MOVE int* data(MOVE int* d1, MOVE int* d2);