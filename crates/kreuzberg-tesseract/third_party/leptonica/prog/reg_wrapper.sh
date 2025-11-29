#!/usr/bin/env bash
#
#  This testing wrapper was written by James Le Cuirot.
#
#  It runs all the programs in AUTO_REG_PROGS in Makefile.am
#  when the command 'make check' is invoked.  This same set can
#  be run by doing:
#      alltests_reg generate
#      alltests_reg compare
#
#  Some of the tests require gnuplot.  These tests, listed below,
#  are skipped if gnuplot is not available.  You can determine if a
#  test requires gnuplot, if any of these situations is true:
#   * a function starting with "gplot" is called
#   * a function starting with "boxaPlot" is called
#   * a function starting with "pixCompare" is called
#   * the function pixItalicWords() is called
#   * the function pixWordMaskByDilation() is called
#
#  The wrapper receives several parameters in this form:
#      path/to/source/config/test-driver <TEST DRIVER ARGS> -- ./foo_reg
#
#  Shell trickery is used to strip off the final parameter and
#  transform the invocation into this.
#      path/to/source/config/test-driver <TEST DRIVER ARGS>
#      -- /bin/sh -c "cd \"path/to/source/prog\" &&
#      \"path/to/build/prog/\"./foo_reg generate &&
#      \"path/to/build/prog/\"./foo_reg compare"
#
#  This also allows testing when you build in a different directory
#  from the install directory, and the logs still get written to
#  the build directory.

eval TEST=\${${#}}

TEST_NAME="${TEST##*/}"
TEST_NAME="${TEST_NAME%_reg*}"

case "${TEST_NAME}" in
    baseline|boxa[1234]|colormask|colorspace|crop|dna|enhance|extrema|fpix1|hash|italic|kernel|nearline|numa[123]|pixa1|projection|rank|rankbin|rankhisto|wordboxes)
        GNUPLOT=$(which gnuplot || which wgnuplot)

        if [ -z "${GNUPLOT}" ] || [ -n "$(${GNUPLOT} -e 'set terminal png' 2>&1)" ] ; then
            exec ${@%${TEST}} /bin/sh -c "exit 77"
        fi
esac

exec ${@%${TEST}} /bin/sh -c "cd \"${srcdir}\" && \"${PWD}/\"${TEST} generate && \"${PWD}/\"${TEST} compare"
