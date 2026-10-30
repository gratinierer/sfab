# CDK: wire the REST API to the Lambda

Label: ready-for-agent

Build the REST API from `openapi/identity.yaml` with `SpecRestApi` in the Python CDK,
as described in `spec.md` "Architecture". Filed from review finding R12.

## Traps to handle

- **No invoke permission:** `SpecRestApi` doesn't add a Lambda permission, because it
  doesn't know the integrations inside the OpenAPI file. Grant API Gateway the right to
  invoke the API Lambda explicitly, or every route fails.
- **Edits may not deploy:** CDK's automatic deployment ID hashes the API model, which
  `SpecRestApi` doesn't build from code, so later `identity.yaml` edits might never reach
  the stage. Pass the OpenAPI definition to `addToLogicalId()` on the deployment, as the
  CDK docs recommend. This hasn't been tested on a real deploy.

## Done when

- Every route reaches the API Lambda after a first deploy.
- An edit to `identity.yaml` reaches the `v1` stage on the next `cdk deploy`.
- A CDK assertion test covers the invoke permission.

## Sources

- <https://docs.aws.amazon.com/cdk/api/v2/docs/aws-cdk-lib.aws_apigateway-readme.html>
- <https://github.com/aws/aws-cdk/blob/main/packages/aws-cdk-lib/aws-apigateway/lib/restapi.ts>
