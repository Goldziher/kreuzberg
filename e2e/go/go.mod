module github.com/xberg-io/xberg/packages/go/e2e

go 1.26

require (
	github.com/stretchr/testify v1.12.1
	github.com/xberg-io/xberg/packages/go v1.3.11
)

require go.yaml.in/yaml/v3 v3.0.5 // indirect

replace github.com/xberg-io/xberg/packages/go => ../../packages/go
