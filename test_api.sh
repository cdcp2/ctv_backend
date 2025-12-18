#!/bin/bash

# --- CONFIGURACIÓN ---
API_URL="http://localhost:3000/api"
# Credenciales de admin existentes (obligatorio)
ADMIN_EMAIL="${ADMIN_EMAIL:?Define ADMIN_EMAIL en el entorno}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:?Define ADMIN_PASSWORD en el entorno}"
# Generamos variables aleatorias para esta ejecución
TIMESTAMP=$(date +%s)
RANDOM_ID=$((1 + $RANDOM % 1000))
EDITOR1_EMAIL="editor1_${TIMESTAMP}@ctv.com"
EDITOR2_EMAIL="editor2_${TIMESTAMP}@ctv.com"
EDITOR_PASSWORD="passwordSegura123"
IMAGE_FILE="test.png"
VIDEO_URL_1="https://www.youtube.com/watch?v=dQw4w9WgXcQ"
VIDEO_URL_2="https://www.youtube.com/watch?v=o-YBDTqX_ZU"

# Colores
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}--- 🚀 INICIANDO PRUEBAS DE INTEGRACIÓN CTV ---${NC}"
echo "ID de Ejecución: $TIMESTAMP"

# 0. VERIFICAR IMAGEN
if [ ! -f "$IMAGE_FILE" ]; then
    echo -e "${RED}❌ Error: No encuentro el archivo '$IMAGE_FILE' en esta carpeta.${NC}"
    echo "Por favor, pon una imagen llamada '$IMAGE_FILE' aquí antes de correr el test."
    exit 1
fi

# 1. LOGIN ADMIN
echo -e "\n🔑 [1/8] Login admin..."
ADMIN_LOGIN_RES=$(curl -s -X POST "$API_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$ADMIN_EMAIL\", \"password\": \"$ADMIN_PASSWORD\"}")
ADMIN_TOKEN=$(echo "$ADMIN_LOGIN_RES" | jq -r '.token')
if [ "$ADMIN_TOKEN" == "null" ] || [ -z "$ADMIN_TOKEN" ]; then
    echo -e "${RED}❌ Error: Login admin fallido. Revisa ADMIN_EMAIL/ADMIN_PASSWORD y que exista en DB.${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Token admin obtenido.${NC}"
fi

# 2. CREAR EDITOR 1
echo -e "\n📡 [2/8] Creando editor 1..."
REGISTER1_RES=$(curl -s -X POST "$API_URL/auth/register" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d "{\"username\": \"editor1_${TIMESTAMP}\", \"email\": \"$EDITOR1_EMAIL\", \"password\": \"$EDITOR_PASSWORD\"}")
if echo "$REGISTER1_RES" | grep -q "existe"; then
    echo -e "${BLUE}➡️  Editor 1 ya existe, seguimos.${NC}"
fi

# 3. LOGIN EDITOR 1
echo -e "\n🔑 [3/8] Login editor 1..."
LOGIN1_RES=$(curl -s -X POST "$API_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$EDITOR1_EMAIL\", \"password\": \"$EDITOR_PASSWORD\"}")
TOKEN1=$(echo "$LOGIN1_RES" | jq -r '.token')
if [ "$TOKEN1" == "null" ] || [ -z "$TOKEN1" ]; then
    echo -e "${RED}❌ Error: Login editor 1 fallido.${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Token editor 1 obtenido.${NC}"
fi

# 4. LISTAR CATEGORÍAS
echo -e "\n📂 [4/8] Verificando Categorías..."
curl -s "$API_URL/categories" | jq -c '.[0:2]' # Solo mostramos las 2 primeras para no llenar la pantalla
echo "..."

# 5. PRUEBA DE SEGURIDAD
echo -e "\n🛡️  [5/8] Probando seguridad (Crear sin token)..."
STATUS_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/articles" \
  -H "Content-Type: application/json" \
  -d '{"title": "Hackeo", "content": "X"}')

if [ "$STATUS_CODE" == "400" ] || [ "$STATUS_CODE" == "401" ]; then
   echo -e "${GREEN}✅ Seguridad OK (Código $STATUS_CODE)${NC}"
else
   echo -e "${RED}❌ ALERTA: Seguridad fallida (Código $STATUS_CODE)${NC}"
fi

# --- NUEVO PASO: SUBIR IMAGEN ---
echo -e "\n📸 [6/8] Subiendo imagen '$IMAGE_FILE'..."
UPLOAD_RES=$(curl -s -X POST "$API_URL/upload" \
  -H "Authorization: Bearer $TOKEN1" \
  -F "image=@$IMAGE_FILE")

# Extraemos la URL usando jq
UPLOADED_URL=$(echo $UPLOAD_RES | jq -r '.url')

if [ "$UPLOADED_URL" == "null" ] || [ -z "$UPLOADED_URL" ]; then
    echo -e "${RED}❌ Falló la subida de imagen: $UPLOAD_RES ${NC}"
    exit 1
else
    echo -e "${BLUE}➡️  Imagen disponible en: $UPLOADED_URL ${NC}"
fi

# 7. CREAR NOTICIA CON EDITOR 1
echo -e "\n📝 [7/8] Creando Noticia como editor 1..."
TITLE="Noticia Automatizada #$RANDOM_ID"
CONTENT="Esta noticia fue generada el $(date) usando la imagen $IMAGE_FILE."

CREATE_RES=$(curl -s -X POST "$API_URL/articles" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN1" \
  -d "{
    \"title\": \"$TITLE\",
    \"content\": \"$CONTENT\",
    \"excerpt\": \"Prueba de integración con imágenes\",
    \"category_id\": 2,
    \"main_image_url\": \"$UPLOADED_URL\",
    \"video_embed_url\": \"$VIDEO_URL_1\"
  }")

# Verificamos si se creó
NEW_ID=$(echo $CREATE_RES | jq -r '.id')
CREATED_VIDEO=$(echo $CREATE_RES | jq -r '.video_embed_url')
SLUG=$(echo $CREATE_RES | jq -r '.slug')

if [ "$NEW_ID" != "null" ]; then
    echo -e "${GREEN}✅ ¡ÉXITO TOTAL! Noticia creada con ID: $NEW_ID ${NC}"
    echo $CREATE_RES | jq .
    if [ "$CREATED_VIDEO" != "$VIDEO_URL_1" ]; then
        echo -e "${RED}❌ video_embed_url no coincide en creación.${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ Error creando noticia.${NC}"
    echo $CREATE_RES
    exit 1
fi

# 8. Incrementar vistas
VIEW1=$(curl -s -X POST "$API_URL/articles/$SLUG/view" | jq -r '.views_count')
VIEW2=$(curl -s -X POST "$API_URL/articles/$SLUG/view" | jq -r '.views_count')
if [ "$VIEW1" != "1" ] || [ "$VIEW2" != "2" ]; then
  echo -e "${RED}❌ Las vistas no se incrementaron como se esperaba (view1=$VIEW1, view2=$VIEW2).${NC}"
  exit 1
else
  echo -e "${GREEN}✅ Vistas incrementadas correctamente (1 -> 2).${NC}"
fi

# 8. EDITAR NOTICIA: PROBAR PERMISOS
echo -e "\n✏️  [8/8] Probando edición con distintos roles..."
UPDATE_EDITOR1_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$API_URL/admin/articles/$NEW_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN1" \
  -d "{\"title\": \"Editada por editor1\", \"video_embed_url\": \"$VIDEO_URL_2\"}")
if [ "$UPDATE_EDITOR1_STATUS" == "200" ]; then
  echo -e "${GREEN}✅ Editor 1 pudo editar su propia noticia.${NC}"
else
  echo -e "${RED}❌ Editor 1 no pudo editar su noticia (código $UPDATE_EDITOR1_STATUS).${NC}"
  exit 1
fi

# Verificar que el video cambió
UPDATED_ARTICLE=$(curl -s "$API_URL/articles/noticia-automatizada-${RANDOM_ID}")
UPDATED_VIDEO=$(echo "$UPDATED_ARTICLE" | jq -r '.video_embed_url')
if [ "$UPDATED_VIDEO" != "$VIDEO_URL_2" ]; then
  echo -e "${RED}❌ video_embed_url no se actualizó correctamente.${NC}"
  exit 1
fi

# Crear editor 2 para probar rechazo
REGISTER2_RES=$(curl -s -X POST "$API_URL/auth/register" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d "{\"username\": \"editor2_${TIMESTAMP}\", \"email\": \"$EDITOR2_EMAIL\", \"password\": \"$EDITOR_PASSWORD\"}")
LOGIN2_RES=$(curl -s -X POST "$API_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$EDITOR2_EMAIL\", \"password\": \"$EDITOR_PASSWORD\"}")
TOKEN2=$(echo "$LOGIN2_RES" | jq -r '.token')

UPDATE_EDITOR2_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$API_URL/admin/articles/$NEW_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN2" \
  -d "{\"title\": \"Editada por editor2\"}")
if [ "$UPDATE_EDITOR2_STATUS" == "403" ]; then
  echo -e "${GREEN}✅ Editor 2 bloqueado al editar noticia de otro (403).${NC}"
else
  echo -e "${RED}❌ Editor 2 no fue bloqueado (código $UPDATE_EDITOR2_STATUS).${NC}"
  exit 1
fi

UPDATE_ADMIN_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$API_URL/admin/articles/$NEW_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d "{\"title\": \"Editada por admin\"}")
if [ "$UPDATE_ADMIN_STATUS" == "200" ]; then
  echo -e "${GREEN}✅ Admin pudo editar noticia ajena.${NC}"
else
  echo -e "${RED}❌ Admin no pudo editar noticia (código $UPDATE_ADMIN_STATUS).${NC}"
  exit 1
fi
