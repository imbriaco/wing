# [lift_this.w](../../../../../examples/tests/valid/lift_this.w) | compile | tf-aws

## inflight.$Closure1.js
```js
module.exports = function({ $f }) {
  class $Closure1 {
    constructor({  }) {
      const $obj = (...args) => this.handle(...args);
      Object.setPrototypeOf($obj, this);
      return $obj;
    }
    async handle() {
      {((cond) => {if (!cond) throw new Error("assertion failed: f.foo() == 21")})(((await $f.foo()) === 21))};
    }
  }
  return $Closure1;
}

```

## inflight.Foo.js
```js
module.exports = function({  }) {
  class Foo {
    constructor({  }) {
    }
    async bar() {
      return this.x;
    }
    async foo() {
      return ((await this.bar()) / 2);
    }
    async $inflight_init() {
      this.x = 42;
    }
  }
  return Foo;
}

```

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
      "value": "[[\"root/Default/Default/test:test\",\"${aws_lambda_function.testtest_Handler_295107CC.arn}\"]]"
    }
  },
  "provider": {
    "aws": [
      {}
    ]
  },
  "resource": {
    "aws_iam_role": {
      "testtest_Handler_IamRole_15693C93": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/test:test/Handler/IamRole",
            "uniqueId": "testtest_Handler_IamRole_15693C93"
          }
        },
        "assume_role_policy": "{\"Version\":\"2012-10-17\",\"Statement\":[{\"Action\":\"sts:AssumeRole\",\"Principal\":{\"Service\":\"lambda.amazonaws.com\"},\"Effect\":\"Allow\"}]}"
      }
    },
    "aws_iam_role_policy": {
      "testtest_Handler_IamRolePolicy_AF0279BD": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/test:test/Handler/IamRolePolicy",
            "uniqueId": "testtest_Handler_IamRolePolicy_AF0279BD"
          }
        },
        "policy": "{\"Version\":\"2012-10-17\",\"Statement\":[{\"Effect\":\"Allow\",\"Action\":\"none:null\",\"Resource\":\"*\"}]}",
        "role": "${aws_iam_role.testtest_Handler_IamRole_15693C93.name}"
      }
    },
    "aws_iam_role_policy_attachment": {
      "testtest_Handler_IamRolePolicyAttachment_ADF4752D": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/test:test/Handler/IamRolePolicyAttachment",
            "uniqueId": "testtest_Handler_IamRolePolicyAttachment_ADF4752D"
          }
        },
        "policy_arn": "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
        "role": "${aws_iam_role.testtest_Handler_IamRole_15693C93.name}"
      }
    },
    "aws_lambda_function": {
      "testtest_Handler_295107CC": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/test:test/Handler/Default",
            "uniqueId": "testtest_Handler_295107CC"
          }
        },
        "environment": {
          "variables": {
            "WING_FUNCTION_NAME": "Handler-c8f4f2a1",
            "WING_TARGET": "tf-aws"
          }
        },
        "function_name": "Handler-c8f4f2a1",
        "handler": "index.handler",
        "publish": true,
        "role": "${aws_iam_role.testtest_Handler_IamRole_15693C93.arn}",
        "runtime": "nodejs18.x",
        "s3_bucket": "${aws_s3_bucket.Code.bucket}",
        "s3_key": "${aws_s3_object.testtest_Handler_S3Object_9F4E28A7.key}",
        "timeout": 30,
        "vpc_config": {
          "security_group_ids": [],
          "subnet_ids": []
        }
      }
    },
    "aws_s3_bucket": {
      "Code": {
        "//": {
          "metadata": {
            "path": "root/Default/Code",
            "uniqueId": "Code"
          }
        },
        "bucket_prefix": "code-c84a50b1-"
      }
    },
    "aws_s3_object": {
      "testtest_Handler_S3Object_9F4E28A7": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/test:test/Handler/S3Object",
            "uniqueId": "testtest_Handler_S3Object_9F4E28A7"
          }
        },
        "bucket": "${aws_s3_bucket.Code.bucket}",
        "key": "<ASSET_KEY>",
        "source": "<ASSET_SOURCE>"
      }
    }
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
    class Foo extends $stdlib.std.Resource {
      constructor(scope, id, ) {
        super(scope, id);
        this._addInflightOps("bar", "foo", "$inflight_init", "x");
      }
      static _toInflightType(context) {
        return $stdlib.core.NodeJsCode.fromInline(`
          require("./inflight.Foo.js")({
          })
        `);
      }
      _toInflight() {
        return $stdlib.core.NodeJsCode.fromInline(`
          (await (async () => {
            const FooClient = ${Foo._toInflightType(this).text};
            const client = new FooClient({
            });
            if (client.$inflight_init) { await client.$inflight_init(); }
            return client;
          })())
        `);
      }
      _registerBind(host, ops) {
        if (ops.includes("foo")) {
          Foo._registerBindObject(this, host, ["bar"]);
        }
        super._registerBind(host, ops);
      }
    }
    class $Closure1 extends $stdlib.std.Resource {
      constructor(scope, id, ) {
        super(scope, id);
        this.display.hidden = true;
        this._addInflightOps("handle", "$inflight_init");
      }
      static _toInflightType(context) {
        return $stdlib.core.NodeJsCode.fromInline(`
          require("./inflight.$Closure1.js")({
            $f: ${context._lift(f)},
          })
        `);
      }
      _toInflight() {
        return $stdlib.core.NodeJsCode.fromInline(`
          (await (async () => {
            const $Closure1Client = ${$Closure1._toInflightType(this).text};
            const client = new $Closure1Client({
            });
            if (client.$inflight_init) { await client.$inflight_init(); }
            return client;
          })())
        `);
      }
      _registerBind(host, ops) {
        if (ops.includes("handle")) {
          $Closure1._registerBindObject(f, host, ["foo"]);
        }
        super._registerBind(host, ops);
      }
    }
    const f = new Foo(this,"Foo");
    this.node.root.new("@winglang/sdk.std.Test",std.Test,this,"test:test",new $Closure1(this,"$Closure1"));
  }
}
const $App = $stdlib.core.App.for(process.env.WING_TARGET);
new $App({ outdir: $outdir, name: "lift_this", rootConstruct: $Root, plugins: $plugins, isTestEnvironment: $wing_is_test }).synth();

```

