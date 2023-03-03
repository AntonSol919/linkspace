.PHONY: all
all: index.html eli5.html lns.html domains.html groups.html download.html why.html

%.html: %.md ./template/*
	pandoc -f markdown $< | cat ./template/head - ./template/tail > $@
