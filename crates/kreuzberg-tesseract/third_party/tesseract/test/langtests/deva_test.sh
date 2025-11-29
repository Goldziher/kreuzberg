#!/bin/bash
# run ./langtests/runlangtests.sh with the root data dir, testname, tessdata-dir, language code and image extension

cd ~/tesseract/test

rm -rf ./langtests/results/*san*
rm -rf ./langtests/results/*Devanagari*
#rm -rf ./langtests/reports/*san-$(date +%F)*
#rm -rf ./langtests/reports/*Devanagari-$(date +%F)*

# Run the tests
./langtests/runlangtests.sh ~/lang-files 4_fast_Devanagari ../tessdata_fast/script Devanagari png
rm -rf ./langtests/results/*Devanagari*
./langtests/runlangtests.sh ~/lang-files 4_best_int_Devanagari ../tessdata/script Devanagari png
rm -rf ./langtests/results/*Devanagari*
./langtests/runlangtests.sh ~/lang-files 4_best_Devanagari ../tessdata_best/script Devanagari png
rm -rf ./langtests/results/*Devanagari*
./langtests/runlangtests.sh ~/lang-files 4_fast_san ../tessdata_fast san png
rm -rf ./langtests/results/*san*
./langtests/runlangtests.sh ~/lang-files 4_best_int_san ../tessdata san png
rm -rf ./langtests/results/*san*
./langtests/runlangtests.sh ~/lang-files 4_best_san ../tessdata_best san png
rm -rf ./langtests/results/*san*

### It takes a while to run.

cd ~/tesseract/test/langtests/
rm -rf ./times.txt
