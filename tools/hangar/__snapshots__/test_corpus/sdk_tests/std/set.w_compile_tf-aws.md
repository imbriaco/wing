# [set.w](../../../../../../examples/tests/sdk_tests/std/set.w) | compile | tf-aws

## main.tf.json
```json
{
  "//": {
    "metadata": {
      "backend": "local",
      "stackName": "root",
      "version": "0.17.0"
    },
    "outputs": {
      "root": {
        "Default": {
          "cloud.TestRunner": {
            "TestFunctionArns": "WING_TEST_RUNNER_FUNCTION_ARNS"
          }
        }
      }
    }
  },
  "output": {
    "WING_TEST_RUNNER_FUNCTION_ARNS": {
      "value": "[]"
    }
  },
  "provider": {
    "aws": [
      {}
    ]
  }
}
```

## preflight.js
```js
const $stdlib = require('@winglang/sdk');
const $outdir = process.env.WING_SYNTH_DIR ?? ".";
const std = $stdlib.std;
const $wing_is_test = process.env.WING_IS_TEST === "true";
class $Root extends $stdlib.std.Resource {
  constructor(scope, id) {
    super(scope, id);
    const mySet = Object.freeze(new Set([1, 2, 3]));
    const myArrayFromSet = Object.freeze([...(mySet)]);
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromSet.at(0) == 1")})(((myArrayFromSet.at(0)) === 1))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromSet.at(1) == 2")})(((myArrayFromSet.at(1)) === 2))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromSet.at(2) == 3")})(((myArrayFromSet.at(2)) === 3))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromSet.length == mySet.size")})((myArrayFromSet.length === mySet.size))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromSet.length == 3")})((myArrayFromSet.length === 3))};
    const myMutSet = new Set(["a", "b", "c"]);
    const myArrayFromMutSet = Object.freeze([...(myMutSet)]);
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromMutSet.at(0) == \"a\"")})(((myArrayFromMutSet.at(0)) === "a"))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromMutSet.at(1) == \"b\"")})(((myArrayFromMutSet.at(1)) === "b"))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromMutSet.at(2) == \"c\"")})(((myArrayFromMutSet.at(2)) === "c"))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromMutSet.length == myMutSet.size")})((myArrayFromMutSet.length === myMutSet.size))};
    {((cond) => {if (!cond) throw new Error("assertion failed: myArrayFromMutSet.length == 3")})((myArrayFromMutSet.length === 3))};
  }
}
const $App = $stdlib.core.App.for(process.env.WING_TARGET);
new $App({ outdir: $outdir, name: "set", rootConstruct: $Root, plugins: $plugins, isTestEnvironment: $wing_is_test }).synth();

```

