# Makefile
CDDE_BASE_NAME       ?= cdde-base
CDDE_DFL_NAME        ?= cdde-dfl
CDDE_DCR_NAME        ?= cdde-dcr
CDDE_DPA_NAME        ?= cdde-dpa
CDDE_CMS_NAME        ?= cdde-cms

DOCKER_ENV           ?= DOCKER_BUILDKIT=1
DOCKER_TAG           ?= 0.1
DOCKER_REGISTRY      ?= docker.io
DOCKER_REPOSITORY    ?= monandkey
DOCKER_BUILD_ARGS    ?= --rm

CDDE_BASE_IMAGE_NAME ?= ${DOCKER_REGISTRY}/${DOCKER_REPOSITORY}/${CDDE_BASE_NAME}:${DOCKER_TAG}
CDDE_DFL_IMAGE_NAME  ?= ${DOCKER_REGISTRY}/${DOCKER_REPOSITORY}/${CDDE_DFL_NAME}:${DOCKER_TAG}
CDDE_DCR_IMAGE_NAME  ?= ${DOCKER_REGISTRY}/${DOCKER_REPOSITORY}/${CDDE_DCR_NAME}:${DOCKER_TAG}
CDDE_DPA_IMAGE_NAME  ?= ${DOCKER_REGISTRY}/${DOCKER_REPOSITORY}/${CDDE_DPA_NAME}:${DOCKER_TAG}
CDDE_CMS_IMAGE_NAME  ?= ${DOCKER_REGISTRY}/${DOCKER_REPOSITORY}/${CDDE_CMS_NAME}:${DOCKER_TAG}

build-all: build-base build-dfl build-dcr build-dpa build-cms

.PHONY: build-base
build-base:
	${DOCKER_ENV} docker build ${DOCKER_BUILD_ARGS} \
		--tag ${CDDE_BASE_IMAGE_NAME} \
		--file ./images/${CDDE_BASE_NAME}/Dockerfile \
		./images/${CDDE_BASE_NAME}

.PHONY: build-dfl
build-dfl: build-base
	${DOCKER_ENV} docker build ${DOCKER_BUILD_ARGS} \
		--tag ${CDDE_DFL_IMAGE_NAME} \
		--file ./images/${CDDE_DFL_NAME}/Dockerfile \
		--build-arg REGISTRY=${DOCKER_REGISTRY} \
		--build-arg REPOSITORY=${DOCKER_REPOSITORY} \
		--build-arg TAG=${DOCKER_TAG} \
		./images/${CDDE_DFL_NAME}

.PHONY: build-dcr
build-dcr: build-base
	${DOCKER_ENV} docker build ${DOCKER_BUILD_ARGS} \
		--tag ${CDDE_DCR_IMAGE_NAME} \
		--file ./images/${CDDE_DCR_NAME}/Dockerfile \
		--build-arg REGISTRY=${DOCKER_REGISTRY} \
		--build-arg REPOSITORY=${DOCKER_REPOSITORY} \
		--build-arg TAG=${DOCKER_TAG} \
		./images/${CDDE_DCR_NAME}

.PHONY: build-dpa
build-dpa: build-base
	${DOCKER_ENV} docker build ${DOCKER_BUILD_ARGS} \
		--tag ${CDDE_DPA_IMAGE_NAME} \
		--file ./images/${CDDE_DPA_NAME}/Dockerfile \
		--build-arg REGISTRY=${DOCKER_REGISTRY} \
		--build-arg REPOSITORY=${DOCKER_REPOSITORY} \
		--build-arg TAG=${DOCKER_TAG} \
		./images/${CDDE_DPA_NAME}

.PHONY: build-cms
build-cms: build-base
	${DOCKER_ENV} docker build ${DOCKER_BUILD_ARGS} \
		--tag ${CDDE_CMS_IMAGE_NAME} \
		--file ./images/${CDDE_CMS_NAME}/Dockerfile \
		--build-arg REGISTRY=${DOCKER_REGISTRY} \
		--build-arg REPOSITORY=${DOCKER_REPOSITORY} \
		--build-arg TAG=${DOCKER_TAG} \
		./images/${CDDE_CMS_NAME}

clean:
	docker rmi ${CDDE_BASE_IMAGE_NAME}
	docker rmi ${CDDE_DFL_IMAGE_NAME}
	docker rmi ${CDDE_DCR_IMAGE_NAME}
	docker rmi ${CDDE_DPA_IMAGE_NAME}
	docker rmi ${CDDE_CMS_IMAGE_NAME}
