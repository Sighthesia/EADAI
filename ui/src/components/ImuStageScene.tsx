import { Canvas, useThree } from '@react-three/fiber'
import { useEffect, useMemo } from 'react'
import * as THREE from 'three'
import type {
  ImuCoordinateScene,
  ImuMotionTrajectory,
  ImuWorldPoint,
  OrientationState,
} from '../lib/imu-view'

const WORLD_AXIS_COLORS = {
  x: '#4FC3F7',
  y: '#C792EA',
  z: '#F78C6C',
} as const

const WORLD_LINE_BASE_AXIS = new THREE.Vector3(0, 1, 0)

const WORLD_CUBE_SIZE: [number, number, number] = [0.82, 0.42, 1.18]

type ImuStageSceneProps = {
  zoom: number
  coordinateScene: ImuCoordinateScene
  orientation: OrientationState
  trajectory: ImuMotionTrajectory | null
  axisColors: Record<'accelX' | 'accelY' | 'accelZ', string>
}

export function ImuStageScene({
  zoom,
  coordinateScene,
  orientation,
  trajectory,
  axisColors,
}: ImuStageSceneProps) {
  const currentAccent = trajectory ? axisColors[trajectory.points[trajectory.points.length - 1]!.dominantAxis] : WORLD_AXIS_COLORS.z

  return (
    <Canvas
      className="imu-stage-canvas imu-stage-surface"
      orthographic
      camera={{ position: [7.5, 6.2, 7.5], zoom: 46 * zoom, near: 0.1, far: 100 }}
      dpr={[1, 1.5]}
      gl={{ antialias: true, alpha: true, powerPreference: 'high-performance' }}
      frameloop="always"
    >
      <CameraRig zoom={zoom} />
      <color attach="background" args={['#090d13']} />
      <ambientLight intensity={0.95} />
      <directionalLight position={[5, 8, 6]} intensity={1.4} />
      <directionalLight position={[-6, 3, -4]} intensity={0.45} color="#7aa2ff" />

      <WorldAxes axes={coordinateScene.axes} />
      <WorldGrid lines={coordinateScene.grid} />
      <MotionTrail trajectory={trajectory} axisColors={axisColors} />
      <CurrentCube orientation={orientation} accent={currentAccent} />
    </Canvas>
  )
}

function CameraRig({ zoom }: { zoom: number }) {
  const { camera, invalidate } = useThree()

  useEffect(() => {
    if (camera instanceof THREE.OrthographicCamera) {
      camera.position.set(7.5, 6.2, 7.5)
      camera.lookAt(0, 0, 0)
      camera.zoom = 46 * zoom
      camera.updateProjectionMatrix()
      invalidate()
    }
  }, [camera, invalidate, zoom])

  return null
}

function WorldAxes({ axes }: { axes: ImuCoordinateScene['axes'] }) {
  return (
    <group>
      {axes.map((axis) => (
        <WorldLine key={axis.key} from={axis.from} to={axis.to} color={axis.color} opacity={axis.opacity} width={0.038} />
      ))}
    </group>
  )
}

function WorldGrid({ lines }: { lines: ImuCoordinateScene['grid'] }) {
  return (
    <group>
      {lines.map((line) => (
        <WorldLine
          key={line.key}
          from={line.from}
          to={line.to}
          color={line.color}
          opacity={line.opacity}
          width={line.kind === 'major' ? 0.022 : 0.014}
        />
      ))}
    </group>
  )
}

function MotionTrail({
  trajectory,
  axisColors,
}: {
  trajectory: ImuMotionTrajectory | null
  axisColors: Record<'accelX' | 'accelY' | 'accelZ', string>
}) {
  if (!trajectory || trajectory.points.length < 2) {
    return null
  }

  return (
    <group>
      {trajectory.points.slice(1).map((point, index) => {
        const previous = trajectory.points[index]!
        const age = index / Math.max(trajectory.points.length - 1, 1)
        const opacity = Math.max(0.12, 0.2 + age * 0.58 + point.intensity * 0.02)
        const width = Math.max(0.04, 0.11 - age * 0.04)
        return (
          <WorldLine
            key={`${previous.timestampMs}-${point.timestampMs}`}
            from={previous.position}
            to={point.position}
            color={axisColors[point.dominantAxis]}
            opacity={opacity}
            width={width}
          />
        )
      })}
    </group>
  )
}

function CurrentCube({
  orientation,
  accent,
}: {
  orientation: OrientationState
  accent: string
}) {
  const quaternion = useMemo(() => toQuaternion(orientation), [orientation])
  const edgeGeometry = useMemo(() => {
    const boxGeometry = new THREE.BoxGeometry(...WORLD_CUBE_SIZE)
    const edgesGeometry = new THREE.EdgesGeometry(boxGeometry)
    boxGeometry.dispose()
    return edgesGeometry
  }, [])
  useEffect(() => () => edgeGeometry.dispose(), [edgeGeometry])

  return (
    <group position={[0, 0, 0]} quaternion={quaternion}>
      <mesh>
        <boxGeometry args={WORLD_CUBE_SIZE} />
        <meshStandardMaterial attach="material" color="#99a7bb" transparent opacity={0.9} roughness={0.35} metalness={0.12} />
      </mesh>
      <lineSegments geometry={edgeGeometry}>
        <lineBasicMaterial attach="material" color="#f0f5fb" transparent opacity={0.78} />
      </lineSegments>
      <mesh position={[0, 0, 0.01]}>
        <sphereGeometry args={[0.09, 16, 16]} />
        <meshStandardMaterial attach="material" color={accent} emissive={accent} emissiveIntensity={0.9} transparent opacity={0.95} />
      </mesh>
      <mesh position={[0, 0, 0]}>
        <sphereGeometry args={[0.23, 18, 18]} />
        <meshBasicMaterial attach="material" color={accent} transparent opacity={0.12} />
      </mesh>
    </group>
  )
}

function WorldLine({ from, to, color, opacity, width = 0.035 }: { from: ImuWorldPoint; to: ImuWorldPoint; color: string; opacity: number; width?: number }) {
  const start = useMemo(() => toVector3(from), [from])
  const end = useMemo(() => toVector3(to), [to])
  const direction = useMemo(() => new THREE.Vector3().subVectors(end, start), [end, start])
  const length = direction.length()
  const center = useMemo(() => new THREE.Vector3().addVectors(start, end).multiplyScalar(0.5), [end, start])
  const orientation = useMemo(() => {
    if (length <= Number.EPSILON) {
      return new THREE.Quaternion()
    }
    return new THREE.Quaternion().setFromUnitVectors(WORLD_LINE_BASE_AXIS, direction.clone().normalize())
  }, [direction, length])
  const geometry = useMemo(() => {
    const buffer = new THREE.CylinderGeometry(width, width, Math.max(length, 0.001), 8, 1, false)
    return buffer
  }, [length, width])

  useEffect(() => () => geometry.dispose(), [geometry])

  return (
    <mesh geometry={geometry} position={center} quaternion={orientation}>
      <meshStandardMaterial attach="material" color={color} transparent opacity={opacity} roughness={0.3} metalness={0.05} depthWrite={false} />
    </mesh>
  )
}

function toQuaternion(orientation: OrientationState) {
  if (orientation.quaternion) {
    const [w, x, y, z] = orientation.quaternion
    return new THREE.Quaternion(x, y, z, w)
  }

  const roll = degreesToRadians(orientation.rollDeg ?? 0)
  const pitch = degreesToRadians(orientation.pitchDeg ?? 0)
  const yaw = degreesToRadians(orientation.yawDeg ?? 0)
  return new THREE.Quaternion().setFromEuler(new THREE.Euler(pitch, yaw, roll, 'YXZ'))
}

function toVector3(point: ImuWorldPoint) {
  return new THREE.Vector3(point.x, point.y, point.z)
}

function degreesToRadians(value: number) {
  return (value * Math.PI) / 180
}
