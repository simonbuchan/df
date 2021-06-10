#ifndef _MSC_VER
#include_next <stdlib.h>
#else
// hack to include c lib in more recent MSVC versions
#include <../ucrt/stdlib.h>

inline void srand48(long int seedval) { srand(seedval); }
inline long int lrand48() { return rand(); }

#endif