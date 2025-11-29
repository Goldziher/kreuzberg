#!/bin/bash
#
cd ~/tesseract

mkdir -p tessdata_best
mkdir -p tessdata_fast
mkdir -p tessdata_fast/script
mkdir -p tessdata_best/script
mkdir -p tessdata/script

cd ~/tesseract/tessdata
wget -O frk.traineddata https://github.com/tesseract-ocr/tessdata/raw/master/frk.traineddata
wget -O eng.traineddata https://github.com/tesseract-ocr/tessdata/raw/master/eng.traineddata
wget -O osd.traineddata https://github.com/tesseract-ocr/tessdata/raw/master/osd.traineddata
cd script
wget -O Fraktur.traineddata https://github.com/tesseract-ocr/tessdata/raw/master/script/Fraktur.traineddata

cd ~/tesseract/tessdata_best
wget -O frk.traineddata https://github.com/tesseract-ocr/tessdata_best/raw/master/frk.traineddata
wget -O eng.traineddata https://github.com/tesseract-ocr/tessdata_best/raw/master/eng.traineddata
wget -O osd.traineddata https://github.com/tesseract-ocr/tessdata_best/raw/master/osd.traineddata
cd script
wget -O Fraktur.traineddata https://github.com/tesseract-ocr/tessdata_best/raw/master/script/Fraktur.traineddata

cd ~/tesseract/tessdata_fast
wget -O frk.traineddata https://github.com/tesseract-ocr/tessdata_fast/raw/master/frk.traineddata
wget -O eng.traineddata https://github.com/tesseract-ocr/tessdata_fast/raw/master/eng.traineddata
wget -O osd.traineddata https://github.com/tesseract-ocr/tessdata_fast/raw/master/osd.traineddata
cd script
wget -O Fraktur.traineddata https://github.com/tesseract-ocr/tessdata_fast/raw/master/script/Fraktur.traineddata

#
mkdir -p ~/lang-downloads
cd ~/lang-downloads
wget -O frk-jbarth-ubhd.zip http://digi.ub.uni-heidelberg.de/diglitData/v/abbyy11r8-vs-tesseract4.zip
wget -O frk-stweil-gt.zip https://digi.bib.uni-mannheim.de/~stweil/fraktur-gt.zip

#
mkdir -p ~/lang-files
cd ~/lang-files
unzip ~/lang-downloads/frk-jbarth-ubhd.zip -d frk
unzip ~/lang-downloads/frk-stweil-gt.zip -d frk
mkdir -p ./frk-ligatures
cp ./frk/abbyy-vs-tesseract/*.tif ./frk-ligatures/
cp ./frk/gt/*.txt ./frk-ligatures/

cd ./frk-ligatures/
ls -1 *.tif >pages
sed -i -e 's/.tif//g' pages

#
## mkdir -p ~/lang-stopwords
## cd ~/lang-stopwords
## wget -O frk.stopwords.txt https://raw.githubusercontent.com/stopwords-iso/stopwords-de/master/stopwords-de.txt
## cat frk.stopwords.txt |  tr '\n' ' ' >  tmp
## echo "\n" > tmpend
## cat tmp tmpend > frk.stopwords.txt
## cp frk.stopwords.txt Fraktur.stopwords.txt
## rm tmp*
## echo "Check ~/lang-stopwords/frk.stopwords.txt as wordacc uses a space delimited stopwords file, not line delimited."
## echo "Also remove duplicate letters because of . and ,"
#
rm -rf ./results/*frk*

cd ~/tesseract/test/langtests
