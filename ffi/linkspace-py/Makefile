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
	. .env/bin/activate && pdoc linkspace

build: setup_env
	. .env/bin/activate && (  maturin build )

install: build
	pip install --force-reinstall ../../target/wheels/linkspace-*.whl

publish: setup_env
	podman run --rm --env PYPI -v   ../../:/linkspace "messense/manylinux2014-cross:x86_64" sh -c 'cd linkspace/ffi/linkspace-py ;  bash ./publish.sh'
