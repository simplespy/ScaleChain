for i in {40000..40005}
do
	kill -9 $(lsof -ti tcp:$i)
done
for i in {41000..41005}
do
	kill -9 $(lsof -ti tcp:$i)
done

