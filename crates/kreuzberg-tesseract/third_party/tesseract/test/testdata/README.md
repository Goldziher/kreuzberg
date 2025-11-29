## testdata

This repo has files required by Google's unittests for Tesseract.

The unicharset files were provided by Ray Smith for unicharcompress_test.
These seem to be old format unicharsets, having ligatures as unichars.

A number of testdata files required for unittests were not made available
with the test sources. An attempt has been made to recreate similar files.

The files for lstm related tests have been created as follows:


```
src/training/tesstrain.sh \
--fonts_dir ~/.fonts \
--lang eng \
--linedata_only   \
--noextract_font_properties \
--workspace_dir ~/tmp \
--exposures "0" \
--langdata_dir ../langdata_lstm  \
--tessdata_dir ../tessdata \
--output_dir ~/tesseract/test/testdata \
--fontlist "Arial" "Arial Unicode MS" \
--training_text ~/langdata_lstm/eng/eng.training_text \
--maxpages 20 \
--xsize 800

rm ~/tesseract/test/testdata/eng.training_files.txt
rm ~/tesseract/test/testdata/eng/eng.charset_size*.txt

src/training/tesstrain.sh \
--fonts_dir ~/.fonts \
--lang kor \
--linedata_only   \
--noextract_font_properties \
--langdata_dir ../langdata_lstm  \
--tessdata_dir ../tessdata \
--output_dir ~/tesseract/test/testdata \
--fontlist "Arial Unicode MS" \
--maxpages 15 \
--xsize 800

rm ~/tesseract/test/testdata/kor.training_files.txt
rm ~/tesseract/test/testdata/kor/kor.charset_size*.txt

src/training/tesstrain.sh \
--fonts_dir ~/.fonts \
--lang kan \
--linedata_only   \
--noextract_font_properties \
--langdata_dir ../langdata_lstm  \
--tessdata_dir ../tessdata \
--output_dir ~/tesseract/test/testdata \
--fontlist "Arial Unicode MS" \
--maxpages 10 --xsize 800

rm ~/tesseract/test/testdata/kan.training_files.txt
rm ~/tesseract/test/testdata/kan/kan.charset_size*.txt

src/training/tesstrain.sh \
--fonts_dir ~/.fonts \
--lang deu \
--linedata_only   \
--noextract_font_properties \
--langdata_dir ../langdata_lstm  \
--tessdata_dir ../tessdata \
--output_dir ~/tesseract/test/testdata \
--fontlist "Arial Unicode MS" \
--maxpages 10 --xsize 800

rm ~/tesseract/test/testdata/deu.training_files.txt
rm ~/tesseract/test/testdata/deu/deu.charset_size*.txt

src/training/tesstrain.sh \
--fonts_dir ~/.fonts \
--lang fra \
--linedata_only   \
--noextract_font_properties \
--langdata_dir ../langdata_lstm  \
--tessdata_dir ../tessdata \
--output_dir ~/tesseract/test/testdata \
--fontlist "Arial Unicode MS" \
--maxpages 10 --xsize 800

rm ~/tesseract/test/testdata/fra.training_files.txt
rm ~/tesseract/test/testdata/fra/fra.charset_size*.txt

```
