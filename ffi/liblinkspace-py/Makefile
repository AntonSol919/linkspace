dev: setup_env
	. .env/bin/activate && ( cargo +nightly build ;  maturin develop )
	$(warn =============================================)
	$(warn   dont forget to `source .env/bin/activate`  )
	$(warn =============================================)

$(abspath .env/pyvenv.cfg):
	python3 -m venv .env
	pip3 install maturin

setup_env: $(abspath .env/pyvenv.cfg)

watch-dev: setup_env
	. .env/bin/activate && ( watchexec -r -e rs -- maturin develop )

doc: build
	. .env/bin/activate && pdoc lkpy

build: setup_env
	. .env/bin/activate && (  maturin build )

install: build
	pip install --force-reinstall ../../target/wheels/lkpy-*.whl
