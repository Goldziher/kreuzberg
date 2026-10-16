#include <errno.h>
#include <stddef.h>

/* WASI has neither a command processor nor host temporary files. ~keep */
int system(const char *command) {
  if (command == NULL) {
    return 0;
  }
  errno = ENOSYS;
  return -1;
}

int mkstemp(char *template) {
  (void)template;
  errno = ENOSYS;
  return -1;
}
