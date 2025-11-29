#!/bin/bash
#
# run langtests/runlangtests.sh with the root ISRI data dir, testname, tessdata-dir, language-code, image-type:

cd ~/tesseract/test

rm -rf ./langtests/results/*frk*
rm -rf ./langtests/results/*Fraktur*
rm -rf ./langtests/reports/*frk-$(date +%F)*
rm -rf ./langtests/reports/*Fraktur-$(date +%F)*

./langtests/runlangtests.sh ~/lang-files 4_fast_Fraktur ../tessdata_fast/script Fraktur tif
rm -rf ./langtests/results/*Fraktur*
./langtests/runlangtests.sh ~/lang-files 4_best_int_Fraktur ../tessdata/script Fraktur tif
rm -rf ./langtests/results/*Fraktur*
./langtests/runlangtests.sh ~/lang-files 4_best_Fraktur ../tessdata_best/script Fraktur tif
rm -rf ./langtests/results/*Fraktur*
./langtests/runlangtests.sh ~/lang-files 4_fast_frk ../tessdata_fast frk tif
rm -rf ./langtests/results/*frk*
./langtests/runlangtests.sh ~/lang-files 4_best_int_frk ../tessdata frk tif
rm -rf ./langtests/results/*frk*
./langtests/runlangtests.sh ~/lang-files 4_best_frk ../tessdata_best frk tif
rm -rf ./langtests/results/*frk*

### It takes a while to run.

mkdir -p ~/tesseract/tessdata_contrib
cd ~/tesseract/test
wget -O ~/tesseract/tessdata_contrib/frk.traineddata https://github.com/Shreeshrii/tessdata_shreetest/raw/master/frk.traineddata
./langtests/runlangtests.sh ~/lang-files 4_shreetest_frk ~/tesseract/tessdata_contrib frk tif
rm -rf ./langtests/results/*frk*

wget -O ~/tesseract/tessdata_contrib/frk.traineddata https://github.com/Shreeshrii/tessdata_fraktur/raw/master/frk-plus-Fraktur-3000.traineddata
./langtests/runlangtests.sh ~/lang-files 4_frk-plus-Fraktur-3000 ~/tesseract/tessdata_contrib frk tif
rm -rf ./langtests/results/*frk*

wget -O ~/tesseract/tessdata_contrib/frk.traineddata https://github.com/Shreeshrii/tessdata_fraktur/raw/master/frk-plus-Fraktur-52500.traineddata
./langtests/runlangtests.sh ~/lang-files 4_frk-plus-Fraktur-52500 ~/tesseract/tessdata_contrib frk tif
rm -rf ./langtests/results/*frk*

cd ~/tesseract/test/langtests/
rm -rf ./times.txt
