.PHONY: all mineweeper

all: index.html mineweeper-1.html mineweeper-2.html bash.exchange.html imageboard.html linkmail.py.html tips_and_tricks.html

%.html: %.md
	pandoc --embed-resources --standalone $< > $@

%.html: %.org ../utils.org
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-html-export-to-html))"

clean:
	rm *.html
