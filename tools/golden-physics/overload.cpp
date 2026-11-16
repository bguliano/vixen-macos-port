#include <math.h>
#include <type_traits>
static_assert(std::is_same<decltype(sin(1.0f)), float>::value, "sin(float) must be the float overload");
static_assert(std::is_same<decltype(cos(1.0f)), float>::value, "cos(float) must be the float overload");
static_assert(std::is_same<decltype(sin(1.0)), double>::value, "sin(double)");
int main() { return 0; }
