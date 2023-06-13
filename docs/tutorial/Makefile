all: index.html mineweeper
mineweeper: ./mineweeper/01.html ./mineweeper/02.html ./bash.exchange.html ./imageboard.html

%.html: %.md
	pandoc --standalone $< > $@

%.html: %.org
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-html-export-to-html))"
