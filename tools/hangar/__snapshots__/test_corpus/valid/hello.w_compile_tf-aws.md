# [hello.w](../../../../../examples/tests/valid/hello.w) | compile | tf-aws

## inflight.$Closure1.js
```js
module.exports = function({ $bucket }) {
  class $Closure1 {
    constructor({  }) {
      const $obj = (...args) => this.handle(...args);
      Object.setPrototypeOf($obj, this);
      return $obj;
    }
    async handle(message) {
      (await $bucket.put("wing.txt",String.raw({ raw: ["Hello, ", ""] }, message)));
    }
  }
  return $Closure1;
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
      "value": "[]"
    }
  },
  "provider": {
    "aws": [
      {}
    ]
  },
  "resource": {
    "aws_iam_role": {
      "cloudQueue-SetConsumer-cdafee6e_IamRole_2548D828": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue-SetConsumer-cdafee6e/IamRole",
            "uniqueId": "cloudQueue-SetConsumer-cdafee6e_IamRole_2548D828"
          }
        },
        "assume_role_policy": "{\"Version\":\"2012-10-17\",\"Statement\":[{\"Action\":\"sts:AssumeRole\",\"Principal\":{\"Service\":\"lambda.amazonaws.com\"},\"Effect\":\"Allow\"}]}"
      }
    },
    "aws_iam_role_policy": {
      "cloudQueue-SetConsumer-cdafee6e_IamRolePolicy_37133937": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue-SetConsumer-cdafee6e/IamRolePolicy",
            "uniqueId": "cloudQueue-SetConsumer-cdafee6e_IamRolePolicy_37133937"
          }
        },
        "policy": "{\"Version\":\"2012-10-17\",\"Statement\":[{\"Action\":[\"sqs:ReceiveMessage\",\"sqs:ChangeMessageVisibility\",\"sqs:GetQueueUrl\",\"sqs:DeleteMessage\",\"sqs:GetQueueAttributes\"],\"Resource\":[\"${aws_sqs_queue.cloudQueue.arn}\"],\"Effect\":\"Allow\"},{\"Action\":[\"s3:PutObject*\",\"s3:Abort*\"],\"Resource\":[\"${aws_s3_bucket.cloudBucket.arn}\",\"${aws_s3_bucket.cloudBucket.arn}/*\"],\"Effect\":\"Allow\"}]}",
        "role": "${aws_iam_role.cloudQueue-SetConsumer-cdafee6e_IamRole_2548D828.name}"
      }
    },
    "aws_iam_role_policy_attachment": {
      "cloudQueue-SetConsumer-cdafee6e_IamRolePolicyAttachment_45079F65": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue-SetConsumer-cdafee6e/IamRolePolicyAttachment",
            "uniqueId": "cloudQueue-SetConsumer-cdafee6e_IamRolePolicyAttachment_45079F65"
          }
        },
        "policy_arn": "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
        "role": "${aws_iam_role.cloudQueue-SetConsumer-cdafee6e_IamRole_2548D828.name}"
      }
    },
    "aws_lambda_event_source_mapping": {
      "cloudQueue_EventSourceMapping_41814136": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue/EventSourceMapping",
            "uniqueId": "cloudQueue_EventSourceMapping_41814136"
          }
        },
        "batch_size": 1,
        "event_source_arn": "${aws_sqs_queue.cloudQueue.arn}",
        "function_name": "${aws_lambda_function.cloudQueue-SetConsumer-cdafee6e.function_name}"
      }
    },
    "aws_lambda_function": {
      "cloudQueue-SetConsumer-cdafee6e": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue-SetConsumer-cdafee6e/Default",
            "uniqueId": "cloudQueue-SetConsumer-cdafee6e"
          }
        },
        "environment": {
          "variables": {
            "BUCKET_NAME_d755b447": "${aws_s3_bucket.cloudBucket.bucket}",
            "WING_FUNCTION_NAME": "cloud-Queue-SetConsumer-cdafee6e-c8eb6a09",
            "WING_TARGET": "tf-aws"
          }
        },
        "function_name": "cloud-Queue-SetConsumer-cdafee6e-c8eb6a09",
        "handler": "index.handler",
        "publish": true,
        "role": "${aws_iam_role.cloudQueue-SetConsumer-cdafee6e_IamRole_2548D828.arn}",
        "runtime": "nodejs18.x",
        "s3_bucket": "${aws_s3_bucket.Code.bucket}",
        "s3_key": "${aws_s3_object.cloudQueue-SetConsumer-cdafee6e_S3Object_8868B9FB.key}",
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
      },
      "cloudBucket": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Bucket/Default",
            "uniqueId": "cloudBucket"
          }
        },
        "bucket_prefix": "cloud-bucket-c87175e7-",
        "force_destroy": false
      }
    },
    "aws_s3_bucket_public_access_block": {
      "cloudBucket_PublicAccessBlock_5946CCE8": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Bucket/PublicAccessBlock",
            "uniqueId": "cloudBucket_PublicAccessBlock_5946CCE8"
          }
        },
        "block_public_acls": true,
        "block_public_policy": true,
        "bucket": "${aws_s3_bucket.cloudBucket.bucket}",
        "ignore_public_acls": true,
        "restrict_public_buckets": true
      }
    },
    "aws_s3_bucket_server_side_encryption_configuration": {
      "cloudBucket_Encryption_77B6AEEF": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Bucket/Encryption",
            "uniqueId": "cloudBucket_Encryption_77B6AEEF"
          }
        },
        "bucket": "${aws_s3_bucket.cloudBucket.bucket}",
        "rule": [
          {
            "apply_server_side_encryption_by_default": {
              "sse_algorithm": "AES256"
            }
          }
        ]
      }
    },
    "aws_s3_object": {
      "cloudQueue-SetConsumer-cdafee6e_S3Object_8868B9FB": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue-SetConsumer-cdafee6e/S3Object",
            "uniqueId": "cloudQueue-SetConsumer-cdafee6e_S3Object_8868B9FB"
          }
        },
        "bucket": "${aws_s3_bucket.Code.bucket}",
        "key": "<ASSET_KEY>",
        "source": "<ASSET_SOURCE>"
      }
    },
    "aws_sqs_queue": {
      "cloudQueue": {
        "//": {
          "metadata": {
            "path": "root/Default/Default/cloud.Queue/Default",
            "uniqueId": "cloudQueue"
          }
        },
        "name": "cloud-Queue-c86e03d8"
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
const cloud = require('@winglang/sdk').cloud;
class $Root extends $stdlib.std.Resource {
  constructor(scope, id) {
    super(scope, id);
    class $Closure1 extends $stdlib.std.Resource {
      constructor(scope, id, ) {
        super(scope, id);
        this.display.hidden = true;
        this._addInflightOps("handle", "$inflight_init");
      }
      static _toInflightType(context) {
        return $stdlib.core.NodeJsCode.fromInline(`
          require("./inflight.$Closure1.js")({
            $bucket: ${context._lift(bucket)},
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
          $Closure1._registerBindObject(bucket, host, ["put"]);
        }
        super._registerBind(host, ops);
      }
    }
    const bucket = this.node.root.newAbstract("@winglang/sdk.cloud.Bucket",this,"cloud.Bucket");
    const queue = this.node.root.newAbstract("@winglang/sdk.cloud.Queue",this,"cloud.Queue");
    (queue.setConsumer(new $Closure1(this,"$Closure1")));
  }
}
const $App = $stdlib.core.App.for(process.env.WING_TARGET);
new $App({ outdir: $outdir, name: "hello", rootConstruct: $Root, plugins: $plugins, isTestEnvironment: $wing_is_test }).synth();

```

