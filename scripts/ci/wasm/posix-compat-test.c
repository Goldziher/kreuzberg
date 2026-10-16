#include <assert.h>
#include <errno.h>
#include <stddef.h>
#include <string.h>

int xberg_test_system(const char *command);
int xberg_test_mkstemp(char *template);

int main(void) {
  char template[] = "/tmp/xberg-unsupported-XXXXXX";
  const char original[] = "/tmp/xberg-unsupported-XXXXXX";

  errno = 0;
  assert(xberg_test_system(NULL) == 0);
  assert(errno == 0);

  assert(xberg_test_system("must never execute") == -1);
  assert(errno == ENOSYS);

  errno = 0;
  assert(xberg_test_mkstemp(template) == -1);
  assert(errno == ENOSYS);
  assert(memcmp(template, original, sizeof(template)) == 0);
  return 0;
}
