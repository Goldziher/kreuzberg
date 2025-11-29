#!/bin/bash
####
# Get the images for testing
#
rm -rf ~/lang-deva-downloads
mkdir ~/lang-deva-downloads
cd ~/lang-deva-downloads
git clone https://github.com/Shreeshrii/imagessan.git --depth 1 

### 
# Copy and rename files as needed for the evaluation script
#
mkdir -p ~/lang-files
rm -rf  ~/lang-files/san-*
for testset in oldstyle shreelipi fontsamples
do
	cd ~/lang-files
	mkdir -p ./san-$testset
	cp ~/lang-deva-downloads/imagessan/$testset/*.* ./san-$testset/
	cd ./san-$testset/
	for f in *-gt.txt; do mv "$f"  "$(echo "$f" | sed -r 's/-gt//')" ; done
	ls -1 *.png >pages
	sed -i -e 's/.png//g' pages
done

###
# Copy Devanagari stopwords
mkdir -p ~/lang-stopwords
cd ~/lang-stopwords
cp ~/lang-deva-downloads/imagessan/stopwords.txt ~/lang-stopwords/san.stopwords.txt
cp ~/lang-deva-downloads/imagessan/stopwords.txt ~/lang-stopwords/Devanagari.stopwords.txt

### 
# Get the traineddata for testing
cd ~/tesseract              
mkdir -p tessdata_best
mkdir -p tessdata_fast
mkdir -p tessdata_fast/script
mkdir -p tessdata_best/script
mkdir -p tessdata/script
#
cd ~/tesseract
cd ./tessdata_best
wget -O san.traineddata https://github.com/tesseract-ocr/tessdata_best/raw/master/san.traineddata
cd ./script
wget -O Devanagari.traineddata https://github.com/tesseract-ocr/tessdata_best/raw/master/script/Devanagari.traineddata
cd ~/tesseract
cd ./tessdata_fast
wget -O san.traineddata https://github.com/tesseract-ocr/tessdata_fast/raw/master/san.traineddata
cd ./script
wget -O Devanagari.traineddata https://github.com/tesseract-ocr/tessdata_fast/raw/master/script/Devanagari.traineddata
cd ~/tesseract
cd ./tessdata
wget -O san.traineddata https://github.com/tesseract-ocr/tessdata/raw/master/san.traineddata
cd ./script
wget -O Devanagari.traineddata https://github.com/tesseract-ocr/tessdata/raw/master/script/Devanagari.traineddata

cd ~/tesseract/test/langtests
