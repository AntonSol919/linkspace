


all: index.html mineweeper
mineweeper: ./mineweeper/01.html ./mineweeper/02.html

%.html: %.md
	pandoc --standalone $< > $@

%.html: %.org
	echo "TODO: Currently not able to make $< outside of emacs"
	exit 1
