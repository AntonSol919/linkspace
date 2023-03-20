.PHONY: all
all: index.html lns.html domains.html groups.html download.html why.html basics.html

basics.html: ./template/* ./basics.html.*
	cat ./template/head ./basics.html.* ./template/tail > basics.html

%.html: %.md ./template/*
	pandoc -f markdown $< | cat ./template/head - ./template/tail > $@
