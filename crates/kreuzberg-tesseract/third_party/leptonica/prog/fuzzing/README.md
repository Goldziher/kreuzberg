# Leptonica fuzzing

This directory contains fuzzing tests for Leptonica.
Each test is in a separate source file *_fuzzer.cc.

Normally these fuzzing tests are run by [OSS-Fuzz](https://oss-fuzz.com/),
but can also be run locally.

## Local build instructions

Local builds require the clang compiler.
Use clang-10 on Debian GNU Linux, which can be installed using

    sudo apt-get install clang-10

To build:

    ./configure CC=clang-10 CXX=clang++-10
    make fuzzers CXX=clang++-10

## Running local fuzzers

Each local fuzzer can be run as in the following example.

    # Show command line syntax.
    ./barcode_fuzzer -help=1

    # Run the fuzzer.
    ./barcode_fuzzer
