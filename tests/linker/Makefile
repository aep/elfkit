SUBDIRS=$(shell find * -maxdepth 1 -type d)

.PHONY: test
test:
	for dir in $(SUBDIRS); do \
		if test -e $$dir/Makefile; \
		then \
		$(MAKE) -C $$dir test || exit; \
		fi; \
	done
	@echo -e "\n======\nPASS"

.PHONY: clean
clean:
	for dir in $(SUBDIRS); do \
		if test -e $$dir/Makefile; \
		then \
		$(MAKE) -C $$dir clean; \
		fi; \
	done


# travis is ubuntu, which is so hopless garbage, i can't be bothered
travis:
	rm -rf cpp-hello
