select armature in object mode -> run script

```
import bpy
for a in bpy.data.actions:
    for fc in a.fcurves:
        if "location" in fc.data_path:
            a.fcurves.remove(fc)

ob = bpy.context.object
for pb in ob.pose.bones:
    pb.location = (0, 0, 0)

bpy.ops.object.transform_apply(scale=True)
```

https://blender.stackexchange.com/questions/143196/apply-scale-to-armature-works-on-rest-position-but-breaks-poses


```
import bpy
for a in bpy.data.actions:
    for fc in a.fcurves:
        if "location" in fc.data_path:
            a.fcurves.remove(fc)

bpy.ops.object.transform_apply(scale=True)
```